use super::test_circuit::TestCircuit;
use halo2_proofs::{arithmetic::FieldExt, dev::MockProver, plonk::Error};
use specs::{CompileTable, ExecutionTable, write_json};

const K: u32 = 18;

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
