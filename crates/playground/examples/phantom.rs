use std::sync::Arc;
use std::sync::Mutex;

use anyhow::Result;
use delphinus_zkwasm::loader::ZkWasmLoader;
use delphinus_zkwasm::runtime::host::default_env::DefaultHostEnvBuilder;
use delphinus_zkwasm::runtime::host::default_env::ExecutionArg;
use pairing_bn256::bn256::Bn256;

fn main() -> Result<()> {
    let wasm = std::fs::read("wasm/phantom.wasm")?;

    let loader = ZkWasmLoader::<Bn256, ExecutionArg, DefaultHostEnvBuilder>::new(
        18,
        wasm,
        vec!["search".to_owned()],
    )?;

    let result = loader.run(ExecutionArg {
        public_inputs: vec![2],
        private_inputs: vec![],
        context_inputs: vec![],
        context_outputs: Arc::new(Mutex::new(vec![])),
    }, (), false, true)?;
    let (circuit, instances) = loader.circuit_with_witness(result)?;
    loader.mock_test(&circuit, &instances)
}
