use anyhow::Result;
use delphinus_zkwasm::circuits::config::MIN_K;
use delphinus_zkwasm::loader::slice::Slices;
use delphinus_zkwasm::loader::TraceBackend;
use delphinus_zkwasm::loader::ZkWasmLoader;
use delphinus_zkwasm::runtime::host::default_env::DefaultHostEnvBuilder;
use delphinus_zkwasm::runtime::host::default_env::ExecutionArg;
use delphinus_zkwasm::runtime::host::HostEnvBuilder;
use delphinus_zkwasm::runtime::monitor::table_monitor::TableMonitor;
use pairing_bn256::bn256::Fr;

const K: u32 = MIN_K;

fn main() -> Result<()> {
    let wasm = std::fs::read("wasm/fibonacci.wasm")?;
    let module = ZkWasmLoader::parse_module(&wasm)?;

    let env = DefaultHostEnvBuilder.create_env(
        K,
        ExecutionArg {
            public_inputs: vec![5],
            private_inputs: vec![],
            context_inputs: vec![],
        },
    );
    let mut monitor = TableMonitor::new(K, &vec![], TraceBackend::Memory, &env);
    let loader = ZkWasmLoader::new(K, env)?;

    let runner = loader.compile(&module, &mut monitor)?;
    let result = loader.run(runner, &mut monitor)?;
    let instances = result.public_inputs_and_outputs::<Fr>();

    let slices = Slices::new(K, monitor.into_tables(), None)?;
    slices.mock_test_all(instances)?;

    Ok(())
}
