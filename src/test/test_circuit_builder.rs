use super::test_circuit::TestCircuit;
use crate::runtime::{WasmInterpreter, WasmRuntime};
use halo2_proofs::pairing::bn256::Fr as Fp;
use halo2_proofs::{arithmetic::FieldExt, dev::MockProver, plonk::Error};
use specs::{write_json, CompileTable, ExecutionTable};
use wasmi::{ImportsBuilder, NopExternals};

const K: u32 = 18;

pub fn test_circuit_noexternal(textual_repr: &str) -> Result<(), Error> {
    let wasm = wabt::wat2wasm(&textual_repr).expect("failed to parse wat");

    let compiler = WasmInterpreter::new();
    let compiled_module = compiler.compile(&wasm, &ImportsBuilder::default()).unwrap();
    let execution_log = compiler
        .run(&mut NopExternals, &compiled_module, "test", vec![])
        .unwrap();
    run_test_circuit::<Fp>(compiled_module.tables, execution_log.tables)
}

pub fn run_test_circuit<F: FieldExt>(
    compile_table: CompileTable,
    execution_table: ExecutionTable,
) -> Result<(), Error> {
    write_json(&compile_table, &execution_table);

    let circuit = TestCircuit::<F>::new(compile_table, execution_table);

    let prover = MockProver::run(K, &circuit, vec![])?;
    assert_eq!(prover.verify(), Ok(()));

    Ok(())
}
