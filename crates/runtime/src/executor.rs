use parking_lot::Mutex;
use std::collections::HashMap;
use wasmtime::{Config, Engine, Instance, Module, ResourceLimiter, Store, StoreLimitsBuilder};

const MAX_RESPONSE_SIZE: usize = 1024 * 1024;
const MAX_MEMORY_BYTES: usize = 64 * 1024 * 1024; // 64 MiB per module

fn pages_needed(required: usize, current: usize) -> u64 {
    let page_size: usize = 65536;
    let additional = required.saturating_sub(current);
    additional.div_ceil(page_size) as u64
}

pub struct WasmExecutor {
    engine: Engine,
    fuel: u64,
    module_cache: Mutex<HashMap<String, Module>>,
}

impl WasmExecutor {
    pub fn new(fuel: u64) -> anyhow::Result<Self> {
        let mut config = Config::new();
        config.consume_fuel(true);

        let engine = Engine::new(&config)?;
        Ok(Self {
            engine,
            fuel,
            module_cache: Mutex::new(HashMap::new()),
        })
    }
    pub fn execute(&self, wasm_path: &str, input: &[u8]) -> anyhow::Result<Vec<u8>> {
        let module = {
            let mut cache = self.module_cache.lock();
            if let Some(module) = cache.get(wasm_path) {
                module.clone()
            } else {
                let module = Module::from_file(&self.engine, wasm_path)?;
                cache.insert(wasm_path.to_string(), module.clone());
                module
            }
        };

        let limits = StoreLimitsBuilder::new()
            .memory_size(MAX_MEMORY_BYTES)
            .build();
        let mut store = Store::new(&self.engine, limits);
        store.limiter(|state| state as &mut dyn ResourceLimiter);
        store.set_fuel(self.fuel)?;

        let instance = Instance::new(&mut store, &module, &[])?;

        let memory = instance
            .get_memory(&mut store, "memory")
            .ok_or_else(|| anyhow::anyhow!("missing memory export"))?;

        let handler = instance.get_typed_func::<(i32, i32), i32>(&mut store, "handler")?;

        let input_ptr = if let Ok(alloc_func) = instance.get_typed_func::<i32, i32>(&mut store, "alloc") {
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
                return Err(anyhow::anyhow!("input exceeds maximum size without alloc export"));
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

        memory.write(&mut store, input_ptr, input)?;

        let out_ptr = handler.call(&mut store, (input_ptr as i32, input.len() as i32))?;
        if out_ptr <= 0 {
            return Err(anyhow::anyhow!(
                "invalid pointer returned from wasm: {}",
                out_ptr
            ));
        }
        let out_ptr = (out_ptr as u32) as usize;

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