#[cfg(test)]
mod tests {
    use crate::{
        circuits::{config::K, TestCircuit},
        foreign::wasm_input_helper::runtime::register_wasm_input_foreign,
        runtime::{host::HostEnv, WasmInterpreter, WasmRuntime},
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

        let wasm = wabt::wat2wasm(&textual_repr).expect("failed to parse wat");

        let compiler = WasmInterpreter::new();
        let mut env = HostEnv::new();
        register_wasm_input_foreign(&mut env);

        let imports = ImportsBuilder::new().with_resolver("env", &env);
        let compiled_module = compiler
            .compile(&wasm, &imports, &env.function_plugin_lookup)
            .unwrap();
        let execution_log = compiler
            .run(&mut env, &compiled_module, "main", vec![])
            .unwrap();

        let circuit = TestCircuit::<Fp>::new(compiled_module.tables, execution_log.tables);

        let prover = MockProver::run(K, &circuit, vec![vec![Fp::from(9)]]).unwrap();
        assert_eq!(prover.verify(), Ok(()));
    }
}
