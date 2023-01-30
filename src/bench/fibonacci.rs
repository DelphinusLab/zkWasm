#[cfg(test)]
mod tests {
    use crate::{circuits::ZkWasmCircuitBuilder, test::test_fibonacci::build_test};
    use halo2_proofs::pairing::bn256::Fr as Fp;
    use wasmi::RuntimeValue;

    #[test]
    fn test_fibonacci() {
        let (execution_result, public_inputs, expected_value) = build_test().unwrap();

        assert_eq!(
            execution_result.result.unwrap(),
            RuntimeValue::I32(expected_value)
        );

        let builder = ZkWasmCircuitBuilder {
            tables: execution_result.tables,
        };

        builder.bench(public_inputs.into_iter().map(|v| Fp::from(v)).collect())
    }
}
