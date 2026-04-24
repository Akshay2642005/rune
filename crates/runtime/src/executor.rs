use anyhow::Context;
use parking_lot::Mutex;
use std::collections::HashMap;
use wasmtime::{
    Config, Engine, Instance, Module, ResourceLimiter, Store, StoreLimitsBuilder, Trap,
};

const MAX_RESPONSE_SIZE: usize = 1024 * 1024;

fn pages_needed(required: usize, current: usize) -> u64 {
    let page_size: usize = 65536;
    let additional = required.saturating_sub(current);
    additional.div_ceil(page_size) as u64
}

pub struct WasmExecutor {
    engine: Engine,
    fuel: u64,
    max_memory_bytes: usize,
    module_cache: Mutex<HashMap<String, Module>>,
}

impl WasmExecutor {
    pub fn new(fuel: u64, max_memory_bytes: u64) -> anyhow::Result<Self> {
        let mut config = Config::new();
        config.consume_fuel(true);

        let engine = Engine::new(&config)?;
        let max_memory_bytes = usize::try_from(max_memory_bytes)
            .context("runtime max_memory_bytes exceeds platform address size")?;

        Ok(Self {
            engine,
            fuel,
            max_memory_bytes,
            module_cache: Mutex::new(HashMap::new()),
        })
    }

    pub fn execute(&self, wasm_path: &str, input: &[u8]) -> anyhow::Result<Vec<u8>> {
        let module = {
            if let Some(module) = self.module_cache.lock().get(wasm_path) {
                module.clone()
            } else {
                let compiled = Module::from_file(&self.engine, wasm_path)?;

                let mut cache = self.module_cache.lock();

                let entry = cache
                    .entry(wasm_path.to_string())
                    .or_insert_with(|| compiled.clone());

                entry.clone()
            }
        };

        let limits = StoreLimitsBuilder::new()
            .memory_size(self.max_memory_bytes)
            .build();
        let mut store = Store::new(&self.engine, limits);
        store.limiter(|state| state as &mut dyn ResourceLimiter);
        store.set_fuel(self.fuel)?;

        let instance = Instance::new(&mut store, &module, &[])?;

        let memory = instance
            .get_memory(&mut store, "memory")
            .ok_or_else(|| anyhow::anyhow!("missing memory export"))?;

        let handler = instance.get_typed_func::<(i32, i32), i32>(&mut store, "handler")?;

        let input_ptr =
            if let Ok(alloc_func) = instance.get_typed_func::<i32, i32>(&mut store, "alloc") {
                let ptr = alloc_func.call(&mut store, input.len() as i32)?;
                if ptr <= 0 {
                    return Err(anyhow::anyhow!("alloc returned invalid pointer"));
                }
                ptr as usize
            } else {
                // Fallback to hardcoded reserved region
                const MAX_INPUT_SIZE: usize = 1024 * 1024; // 1 MiB
                let input_ptr = 8usize;
                if input.len() > MAX_INPUT_SIZE {
                    return Err(anyhow::anyhow!(
                        "input exceeds maximum size without alloc export"
                    ));
                }

                let required = input_ptr + input.len();
                let current = memory.data_size(&store);

                if required > current {
                    let pages = pages_needed(required, current);
                    if pages > 0 {
                        memory.grow(&mut store, pages)?;
                    }
                }

                input_ptr
            };
        let required = input_ptr + input.len();
        let current = memory.data_size(&store);

        if required > current {
            return Err(anyhow::anyhow!(
                "allocated region out of bounds (guest alloc bug)"
            ));
        }

        memory.write(&mut store, input_ptr, input)?;

        let out_ptr = handler.call(&mut store, (input_ptr as i32, input.len() as i32))?;
        if out_ptr <= 0 {
            return Err(anyhow::anyhow!(
                "invalid pointer returned from wasm: {}",
                out_ptr
            ));
        }
        let out_ptr = out_ptr as usize;

        let mut len_buf = [0u8; 4];
        let current = memory.data_size(&store);
        let len_end = out_ptr
            .checked_add(4)
            .ok_or_else(|| anyhow::anyhow!("response pointer overflow"))?;

        if len_end > current {
            return Err(anyhow::anyhow!("response length header out of bounds"));
        }

        memory.read(&mut store, out_ptr, &mut len_buf)?;

        let resp_len = u32::from_le_bytes(len_buf) as usize;
        if resp_len > MAX_RESPONSE_SIZE {
            return Err(anyhow::anyhow!("response too large"));
        }

        let resp_end = len_end
            .checked_add(resp_len)
            .ok_or_else(|| anyhow::anyhow!("response length overflow"))?;

        if resp_end > current {
            return Err(anyhow::anyhow!("response body out of bounds"));
        }
        let mut resp = vec![0u8; resp_len];
        memory.read(&mut store, out_ptr + 4, &mut resp)?;

        Ok(resp)
    }
}

pub fn is_out_of_fuel(error: &anyhow::Error) -> bool {
    error.chain().any(|cause| {
        cause
            .downcast_ref::<Trap>()
            .is_some_and(|trap| matches!(trap, Trap::OutOfFuel))
    })
}
