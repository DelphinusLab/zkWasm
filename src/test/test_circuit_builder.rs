use super::test_circuit::TestCircuit;
use crate::spec::{CompileTable, ExecutionTable};
use halo2_proofs::{arithmetic::FieldExt, dev::MockProver, plonk::Error};

const K: u32 = 5;

pub fn run_test_circuit<F: FieldExt>(
    compile_table: CompileTable,
    execution_table: ExecutionTable,
) -> Result<(), Error> {
    let circuit = TestCircuit::<F>::new(compile_table, execution_table);

    MockProver::run(K, &circuit, vec![])?;

    Ok(())
}
