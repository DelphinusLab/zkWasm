#[cfg(test)]
mod tests {
    use crate::{circuits::ZkWasmCircuitBuilder, test::test_fibonacci::build_test};
    use halo2_proofs::pairing::bn256::Fr as Fp;

    #[test]
    fn test_fibonacci() {
        let (compile_tables, execution_tables, public_inputs) = build_test();

        let builder = ZkWasmCircuitBuilder {
            compile_tables,
            execution_tables,
        };

        builder.bench(public_inputs.into_iter().map(|v| Fp::from(v)).collect())
    }
}
