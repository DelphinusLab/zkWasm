use anyhow::Result;
use delphinus_zkwasm::circuits::MIN_K;
use delphinus_zkwasm::loader::slice::Slices;
use delphinus_zkwasm::loader::ZkWasmLoader;
use delphinus_zkwasm::runtime::host::default_env::DefaultHostEnvBuilder;
use delphinus_zkwasm::runtime::host::default_env::ExecutionArg;
use delphinus_zkwasm::runtime::host::HostEnvBuilder;
use delphinus_zkwasm::runtime::monitor::plugins::table::InMemoryBackend;
use delphinus_zkwasm::runtime::monitor::table_monitor::TableMonitor;
use pairing_bn256::bn256::Fr;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

const K: u32 = MIN_K;

fn main() -> Result<()> {
    let wasm = std::fs::read("wasm/fibonacci.wasm")?;
    let module = ZkWasmLoader::parse_module(&wasm)?;
    let env_builder = DefaultHostEnvBuilder::new(K);

    let env = env_builder.create_env(ExecutionArg {
        public_inputs: vec![5],
        private_inputs: vec![],
        context_inputs: vec![],
        indexed_witness: Rc::new(RefCell::new(HashMap::default())),
        tree_db: None,
    });
    let mut monitor = TableMonitor::new(
        K,
        env_builder.create_flush_strategy(),
        Box::<InMemoryBackend>::default(),
        &vec![],
        &env,
    );
    let loader = ZkWasmLoader::new(K, env)?;

    let runner = loader.compile(&module, &mut monitor)?;
    let result = loader.run(runner, &mut monitor)?;
    let instances = result.public_inputs_and_outputs::<Fr>();

    let slices = Slices::new(K, monitor.into_tables(), None)?;
    slices.mock_test_all(instances)?;

    Ok(())
}
