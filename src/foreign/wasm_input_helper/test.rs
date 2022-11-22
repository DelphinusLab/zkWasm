#[cfg(test)]
mod tests {
    use crate::{
        circuits::{config::K, TestCircuit},
        foreign::wasm_input_helper::runtime::register_wasm_input_foreign,
        runtime::{host::HostEnv, WasmInterpreter, WasmRuntime},
        test::run_test_circuit,
    };

    use halo2_proofs::{dev::MockProver, pairing::bn256::Fr as Fp};
    use wasmi::ImportsBuilder;

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

        let imports = ImportsBuilder::new().with_resolver("env", &env);

        let compiler = WasmInterpreter::new(env.function_plugin_lookup.clone());
        let compiled_module = compiler.compile(&wasm, &imports).unwrap();
        let _ = compiler
            .run(
                &mut env,
                &compiled_module,
                "main",
                public_inputs.clone(),
                vec![],
            )
            .unwrap();

        let circuit = TestCircuit::<Fp>::from_wasm_runtime(&compiler);

        let prover = MockProver::run(
            K,
            &circuit,
            vec![public_inputs.into_iter().map(|v| Fp::from(v)).collect()],
        )
        .unwrap();
        assert_eq!(prover.verify(), Ok(()));
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

        let compiler = WasmInterpreter::new(env.function_plugin_lookup.clone());

        let imports = ImportsBuilder::new().with_resolver("env", &env);
        let compiled_module = compiler.compile(&wasm, &imports).unwrap();
        let _ = compiler
            .run(
                &mut env,
                &compiled_module,
                "main",
                public_inputs.clone(),
                private_inputs,
            )
            .unwrap();

        run_test_circuit(
            compiler.compile_table(),
            compiler.execution_tables(),
            public_inputs.into_iter().map(|v| Fp::from(v)).collect(),
        )
        .unwrap();
    }
}
