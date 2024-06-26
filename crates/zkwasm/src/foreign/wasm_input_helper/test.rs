#[cfg(test)]
mod tests {
    use crate::test::test_circuit_with_env;

    #[test]
    fn test_foreign_wasm_input() {
        let textual_repr = r#" 
                (module
                    (import "env" "wasm_input" (func $wasm_input (param i32) (result i64)))
                    (export "zkwasm" (func $zkwasm))
                    (func $zkwasm (; 1 ;)
                        (call $wasm_input (i32.const 1))
                        (drop)
                    )
                )
            "#;

        let public_inputs = vec![9];
        let private_inputs = vec![];
        let wasm = wabt::wat2wasm(textual_repr).expect("failed to parse wat");

        test_circuit_with_env(
            18,
            wasm,
            "zkwasm".to_string(),
            public_inputs,
            private_inputs,
        )
        .unwrap();
    }

    #[test]
    fn test_foreign_wasm_input_multi_public() {
        let textual_repr = r#"
        (module
            (type (;0;) (func (param i32) (result i64)))
            (type (;1;) (func (result i32)))
            (import "env" "wasm_input" (func (;0;) (type 0)))
            (func (;1;) (type 1) (result i32)
              (local i64)
              i32.const 1
              call 0
              local.set 0
              i32.const 1
              call 0
              i32.wrap_i64
              local.get 0
              i32.wrap_i64
              i32.add)
            (memory (;0;) 2 2)
            (export "memory" (memory 0))
            (export "zkwasm" (func 1)))
        "#;

        let wasm = wabt::wat2wasm(textual_repr).expect("failed to parse wat");

        let private_inputs = vec![];
        let public_inputs = vec![1, 2];

        test_circuit_with_env(
            18,
            wasm,
            "zkwasm".to_string(),
            public_inputs,
            private_inputs,
        )
        .unwrap();
    }
}
