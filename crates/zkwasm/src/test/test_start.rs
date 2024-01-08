mod tests {
    use std::sync::Arc;
    use std::sync::Mutex;

    use halo2_proofs::pairing::bn256::Bn256;

    use crate::loader::ZkWasmLoader;
    use crate::runtime::host::default_env::DefaultHostEnvBuilder;
    use crate::runtime::host::default_env::ExecutionArg;

    #[test]
    fn test_start_mock() {
        let textual_repr = r#"
        (module
            (type (;0;) (func (param i32) (result i64)))
            (type (;1;) (func (param i64)))

            (import "env" "wasm_input" (func $wasm_input (type 0)))
            (import "env" "wasm_output" (func $wasm_output (type 1)))

            (func $start
              i32.const 0
              drop
            )

            (func $zkmain
              i32.const 1
              drop
            )

            (start $start)
            (export "zkmain" (func $zkmain))
           )
        "#;

        let wasm = wabt::wat2wasm(&textual_repr).expect("failed to parse wat");

        let loader =
            ZkWasmLoader::<Bn256, ExecutionArg, DefaultHostEnvBuilder>::new(18, wasm, vec![])
                .unwrap();

        let arg = ExecutionArg {
            public_inputs: vec![],
            private_inputs: vec![],
            context_inputs: vec![],
            context_outputs: Arc::new(Mutex::new(vec![])),
        };

        let result = loader.run(arg, (), false, true).unwrap();

        let (circuit, instances) = loader.circuit_with_witness(result).unwrap();

        loader.mock_test(&circuit, &instances).unwrap()
    }
}
