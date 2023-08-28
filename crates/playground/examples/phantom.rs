use std::cell::RefCell;
use std::rc::Rc;

use anyhow::Result;
use delphinus_zkwasm::loader::ExecutionArg;
use delphinus_zkwasm::loader::ZkWasmLoader;
use pairing_bn256::bn256::Bn256;

fn main() -> Result<()> {
    let wasm = std::fs::read("wasm/phantom.wasm")?;

    let loader = ZkWasmLoader::<Bn256>::new(
        18,
        wasm,
        vec!["search"].into_iter().map(|s| s.to_owned()).collect(),
        None,
    )?;

    let (circuit, instances) = loader.circuit_with_witness(ExecutionArg {
        public_inputs: vec![2],
        private_inputs: vec![],
        context_inputs: vec![],
        context_outputs: Rc::new(RefCell::new(vec![])),
    })?;
    loader.mock_test(&circuit, &instances)
}
