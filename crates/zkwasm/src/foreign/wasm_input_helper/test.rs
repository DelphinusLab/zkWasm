#[cfg(test)]
mod tests {
    use crate::circuits::config::MIN_K;
    use crate::foreign::wasm_input_helper::runtime::register_wasm_input_foreign;
    use crate::runtime::host::host_env::HostEnv;
    use crate::test::test_circuit_with_env;

    #[test]
    fn test_foreign_wasm_input() {
        let textual_repr = r#" 
                (module
                    (import "env" "wasm_input" (func $wasm_input (param i32) (result i64)))
                    (export "main" (func $main))
                    (func $main (; 1 ;)
                        (call $wasm_input (i32.const 1))
                        (drop)
                    )
                )
            "#;

        let k = MIN_K;

        let public_inputs = vec![9];
        let wasm = wabt::wat2wasm(&textual_repr).expect("failed to parse wat");

        let mut env = HostEnv::new(k);
        let wasm_runtime_io = register_wasm_input_foreign(&mut env, public_inputs, vec![]);
        env.finalize();

        test_circuit_with_env(env, wasm_runtime_io, wasm, "main").unwrap();
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
            (export "main" (func 1)))
        "#;

        let wasm = wabt::wat2wasm(&textual_repr).expect("failed to parse wat");

        let private_inputs = vec![];
        let public_inputs = vec![1, 2];

        let mut env = HostEnv::new(MIN_K);
        let wasm_runtime_io = register_wasm_input_foreign(&mut env, public_inputs, private_inputs);
        env.finalize();

        test_circuit_with_env(env, wasm_runtime_io, wasm, "main").unwrap();
    }
}
