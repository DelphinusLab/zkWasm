#[cfg(test)]
mod tests {
    use crate::{
        foreign::wasm_input_helper::runtime::register_wasm_input_foreign,
        runtime::host::host_env::HostEnv, test::test_circuit_with_env,
    };

    use halo2_proofs::pairing::bn256::Fr as Fp;

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

        let public_inputs = vec![9];
        let wasm = wabt::wat2wasm(&textual_repr).expect("failed to parse wat");

        let mut env = HostEnv::new();
        register_wasm_input_foreign(&mut env, public_inputs.clone(), vec![]);
        env.finalize();

        test_circuit_with_env(
            env,
            wasm,
            "main",
            public_inputs.into_iter().map(|v| Fp::from(v)).collect(),
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
            (export "main" (func 1)))
        "#;

        let wasm = wabt::wat2wasm(&textual_repr).expect("failed to parse wat");

        let private_inputs = vec![];
        let public_inputs = vec![1, 2];

        let mut env = HostEnv::new();
        register_wasm_input_foreign(&mut env, public_inputs.clone(), private_inputs.clone());
        env.finalize();

        test_circuit_with_env(
            env,
            wasm,
            "main",
            public_inputs.into_iter().map(|v| Fp::from(v)).collect(),
        )
        .unwrap();
    }
}
