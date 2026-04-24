use wasmtime::{Config, Engine, Instance, Module, Store};

pub struct WasmExecutor {
    engine: Engine,
}

impl WasmExecutor {
    pub fn new() -> anyhow::Result<Self> {
        let mut config = Config::new();
        config.consume_fuel(true);

        let engine = Engine::new(&config)?;
        Ok(Self { engine })
    }
    pub fn execute(&self, wasm_path: &str) -> anyhow::Result<Vec<u8>> {
        let module = Module::from_file(&self.engine, wasm_path)?;

        let mut store = Store::new(&self.engine, ());
        store.set_fuel(10_000)?;

        let instance = Instance::new(&mut store, &module, &[])?;

        let memory = instance
            .get_memory(&mut store, "memory")
            .expect("failed to find memory export");

        let handler = instance.get_typed_func::<(i32, i32), i32>(&mut store, "handler")?;

        let input = b"{}";
        let input_ptr = 8usize; // avoid null pointer

        let required = input_ptr + input.len();
        let current = memory.data_size(&store);

        if required > current {
            let page_size = 65536;
            let pages = required.div_ceil(page_size);
            memory.grow(&mut store, pages as u64)?;
        }

        memory.write(&mut store, input_ptr, input)?;

        let out_ptr = handler.call(&mut store, (input_ptr as i32, input.len() as i32))?;
        let out_ptr = out_ptr as usize;

        let mut len_buf = [0u8; 4];

        let required = out_ptr + 4;
        let current = memory.data_size(&store);

        if required > current {
            let page_size = 65536;
            let additional = required - current;
            let pages = additional.div_ceil(page_size);
            memory.grow(&mut store, pages as u64)?;
        }

        memory.read(&mut store, out_ptr, &mut len_buf)?;

        let resp_len = u32::from_le_bytes(len_buf) as usize;

        let required = out_ptr + 4 + resp_len;
        let current = memory.data_size(&store);

        if required > current {
            let page_size = 65536;
            let additional = required - current;
            let pages = additional.div_ceil(page_size);
            memory.grow(&mut store, pages as u64)?;
        }

        let mut resp = vec![0u8; resp_len];
        memory.read(&mut store, out_ptr + 4, &mut resp)?;

        Ok(resp)
    }
}
