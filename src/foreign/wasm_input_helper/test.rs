#[cfg(test)]
mod tests {
    use crate::{
        circuits::{config::zkwasm_k, TestCircuit},
        foreign::wasm_input_helper::runtime::register_wasm_input_foreign,
        runtime::{host::HostEnv, wasmi_interpreter::Execution, WasmInterpreter, WasmRuntime},
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

        let compiler = WasmInterpreter::new();
        let mut env = HostEnv::new();
        register_wasm_input_foreign(&mut env, public_inputs.clone(), vec![]);

        let imports = ImportsBuilder::new().with_resolver("env", &env);
        let compiled_module = compiler
            .compile(&wasm, &imports, &env.function_plugin_lookup)
            .unwrap();
        let execution_result = compiled_module.run(&mut env, "main").unwrap();

        let circuit = TestCircuit::<Fp>::new(execution_result.tables);

        let prover = MockProver::run(
            zkwasm_k(),
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

        let compiler = WasmInterpreter::new();
        let mut env = HostEnv::new();
        register_wasm_input_foreign(&mut env, public_inputs.clone(), private_inputs.clone());

        let imports = ImportsBuilder::new().with_resolver("env", &env);
        let compiled_module = compiler
            .compile(&wasm, &imports, &env.function_plugin_lookup)
            .unwrap();
        let execution_result = compiled_module.run(&mut env, "main").unwrap();

        let circuit = TestCircuit::<Fp>::new(execution_result.tables);

        let prover = MockProver::run(
            zkwasm_k(),
            &circuit,
            vec![public_inputs.into_iter().map(|v| Fp::from(v)).collect()],
        )
        .unwrap();

        assert_eq!(prover.verify(), Ok(()));
    }
}
