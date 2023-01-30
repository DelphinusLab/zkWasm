#[cfg(test)]
mod tests {
    use crate::circuits::ZkWasmCircuitBuilder;
    use crate::test::test_binary_search::build_test;
    use halo2_proofs::pairing::bn256::Fr as Fp;

    #[test]
    fn test_binary_search() {
        let (execution_result, public_inputs) = build_test().unwrap();

        let builder = ZkWasmCircuitBuilder {
            tables: execution_result,
        };

        builder.bench(public_inputs.into_iter().map(|v| Fp::from(v)).collect())
    }
}
