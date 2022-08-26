#[cfg(test)]
mod tests {
    use halo2_proofs::pairing::bn256::Fr as Fp;
    use specs::types::Value;
    use wasmi::{ImportsBuilder, NopExternals};

    use crate::{
        runtime::{WasmInterpreter, WasmRuntime},
        test::run_test_circuit,
    };

    #[test]
    fn test_fibonacci() {
        let textual_repr = r#"
        (module
            (export "fibonacci" (func $fibonacci))
            (func $fibonacci (; 0 ;) (param $0 i32) (result i32)
             (block $label$0
              (br_if $label$0
               (i32.ne
                (i32.or
                 (local.get $0)
                 (i32.const 1)
                )
                (i32.const 1)
               )
              )
              (return
               (local.get $0)
              )
             )
             (i32.add
              (call $fibonacci
               (i32.add
                (local.get $0)
                (i32.const -1)
               )
              )
              (call $fibonacci
               (i32.add
                (local.get $0)
                (i32.const -2)
               )
              )
             )
            )
           )
        "#;

        let compiler = WasmInterpreter::new();
        let wasm = wabt::wat2wasm(textual_repr).unwrap();

        let compiled_module = compiler.compile(&wasm, &ImportsBuilder::default()).unwrap();
        let execution_log = compiler
            .run(
                &mut NopExternals,
                &compiled_module,
                "fibonacci",
                vec![Value::I32(15)],
            )
            .unwrap();
        run_test_circuit::<Fp>(compiled_module.tables, execution_log.tables).unwrap()
    }
}
