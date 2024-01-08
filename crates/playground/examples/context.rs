use std::sync::Arc;
use std::sync::Mutex;

use anyhow::Result;
use delphinus_zkwasm::loader::ZkWasmLoader;
use delphinus_zkwasm::runtime::host::default_env::DefaultHostEnvBuilder;
use delphinus_zkwasm::runtime::host::default_env::ExecutionArg;
use pairing_bn256::bn256::Bn256;

fn main() -> Result<()> {
    let wasm = std::fs::read("wasm/context.wasm")?;

    let context_in: Vec<u64> = vec![2, 1];
    let context_outputs = Arc::new(Mutex::new(vec![]));

    let loader = ZkWasmLoader::<Bn256, ExecutionArg, DefaultHostEnvBuilder>::new(18, wasm, vec![])?;
    let arg = ExecutionArg {
        public_inputs: vec![],
        private_inputs: vec![],
        context_inputs: context_in,
        context_outputs: context_outputs.clone(),
    };

    let result = loader.run(arg, (), false, true)?;
    let (circuit, instances) = loader.circuit_with_witness(result)?;
    loader.mock_test(&circuit, &instances)?;

    let arg = ExecutionArg {
        public_inputs: vec![],
        private_inputs: vec![],
        context_inputs: context_outputs.lock().unwrap().to_vec(),
        context_outputs: Arc::new(Mutex::new(vec![])),
    };

    let result = loader.run(arg, (), false, true)?;

    let (circuit, instances) = loader.circuit_with_witness(result)?;
    loader.mock_test(&circuit, &instances)?;

    Ok(())
}
