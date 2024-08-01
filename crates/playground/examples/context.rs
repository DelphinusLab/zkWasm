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
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

const K: u32 = MIN_K;

fn main() -> Result<()> {
    let wasm = std::fs::read("wasm/context.wasm")?;
    let module = ZkWasmLoader::parse_module(&wasm)?;

    let context_output = {
        let env = DefaultHostEnvBuilder.create_env(
            K,
            ExecutionArg {
                public_inputs: vec![],
                private_inputs: vec![],
                context_inputs: vec![2, 1],
                indexed_witness: Rc::new(RefCell::new(HashMap::default())),
                tree_db: None,
            },
        );

        let mut monitor = TableMonitor::new(K, &vec![], TraceBackend::Memory, &env);
        let loader = ZkWasmLoader::new(K, env)?;

        let runner = loader.compile(&module, &mut monitor)?;
        let result = loader.run(runner, &mut monitor)?;

        let slices: Slices<Fr> = Slices::new(K, monitor.into_tables(), None)?;
        slices.mock_test_all(result.public_inputs_and_outputs())?;

        result.context_outputs
    };

    {
        let env = DefaultHostEnvBuilder.create_env(
            K,
            ExecutionArg {
                public_inputs: vec![],
                private_inputs: vec![],
                context_inputs: context_output.0,
                indexed_witness: Rc::new(RefCell::new(HashMap::default())),
                tree_db: None,
            },
        );

        let mut monitor = TableMonitor::new(K, &vec![], TraceBackend::Memory, &env);
        let loader = ZkWasmLoader::new(K, env)?;

        let runner = loader.compile(&module, &mut monitor)?;
        let result = loader.run(runner, &mut monitor)?;

        let slices: Slices<Fr> = Slices::new(K, monitor.into_tables(), None)?;
        slices.mock_test_all(result.public_inputs_and_outputs())?;
    }

    Ok(())
}
