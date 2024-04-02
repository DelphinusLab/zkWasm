mod tests {
    use crate::test::test_circuit_with_env;

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

        test_circuit_with_env(wasm, "zkmain".to_string(), vec![], vec![]).unwrap();
    }
}
