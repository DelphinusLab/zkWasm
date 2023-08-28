use std::cell::RefCell;
use std::rc::Rc;

use anyhow::Result;
use delphinus_zkwasm::loader::ExecutionArg;
use delphinus_zkwasm::loader::ZkWasmLoader;
use pairing_bn256::bn256::Bn256;

fn main() -> Result<()> {
    let wasm = std::fs::read("wasm/context.wasm")?;

    let context_in: Vec<u64> = vec![2, 1];
    let context_outputs = Rc::new(RefCell::new(vec![]));

    let loader = ZkWasmLoader::<Bn256>::new(18, wasm, vec![], None)?;
    let arg = ExecutionArg {
        public_inputs: vec![],
        private_inputs: vec![],
        context_inputs: context_in,
        context_outputs: context_outputs.clone(),
    };

    let (circuit, instances) = loader.circuit_with_witness(arg)?;
    loader.mock_test(&circuit, &instances)?;

    let arg = ExecutionArg {
        public_inputs: vec![],
        private_inputs: vec![],
        context_inputs: context_outputs.borrow().to_vec(),
        context_outputs: Rc::new(RefCell::new(vec![])),
    };

    let (circuit, instances) = loader.circuit_with_witness(arg)?;
    loader.mock_test(&circuit, &instances)?;

    Ok(())
}
