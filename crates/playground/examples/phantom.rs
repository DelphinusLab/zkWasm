use std::cell::RefCell;
use std::rc::Rc;

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
        vec!["search"].into_iter().map(|s| s.to_owned()).collect(),
    )?;

    let (circuit, instances) = loader.circuit_with_witness(ExecutionArg {
        public_inputs: vec![2],
        private_inputs: vec![],
        context_inputs: vec![],
        context_outputs: Rc::new(RefCell::new(vec![])),
    })?;
    loader.mock_test(&circuit, &instances)
}
