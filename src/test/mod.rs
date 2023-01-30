use crate::{
    circuits::{config::zkwasm_k, TestCircuit},
    runtime::{wasmi_interpreter::Execution, WasmInterpreter, WasmRuntime},
};

use anyhow::Result;
use halo2_proofs::{arithmetic::FieldExt, dev::MockProver, pairing::bn256::Fr as Fp};
use specs::Tables;
use std::collections::HashMap;
use wasmi::{ImportsBuilder, NopExternals};

mod spec;
pub mod test_binary_search;
pub mod test_fibonacci;

pub fn test_circuit_noexternal(textual_repr: &str) -> Result<()> {
    let wasm = wabt::wat2wasm(&textual_repr).expect("failed to parse wat");

    let compiler = WasmInterpreter::new();
    let compiled_module = compiler
        .compile(&wasm, &ImportsBuilder::default(), &HashMap::default())
        .unwrap();
    let execution_result = compiled_module.run(&mut NopExternals, "test")?;

    run_test_circuit::<Fp>(execution_result.tables, vec![])
}

pub fn run_test_circuit<F: FieldExt>(tables: Tables, public_inputs: Vec<F>) -> Result<()> {
    tables.write_json(None);

    let circuit = TestCircuit::<F>::new(tables);

    let prover = MockProver::run(zkwasm_k(), &circuit, vec![public_inputs])?;
    assert_eq!(prover.verify(), Ok(()));

    Ok(())
}
