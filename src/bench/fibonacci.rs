#[cfg(test)]
mod tests {
    use crate::{circuits::ZkWasmCircuitBuilder, test::test_fibonacci::build_test};
    use halo2_proofs::pairing::bn256::Fr as Fp;

    #[test]
    fn test_fibonacci() {
        let (compiled_module, execution_log, public_inputs) = build_test();

        let builder = ZkWasmCircuitBuilder {
            compile_tables: compiled_module.tables,
            execution_tables: execution_log.tables,
        };

        builder.bench(public_inputs.into_iter().map(|v| Fp::from(v)).collect())
    }
}
