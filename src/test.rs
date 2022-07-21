pub mod test_circuit;
pub mod test_circuit_builder;

pub mod tests {
    use super::test_circuit_builder::run_test_circuit;
    use crate::runtime::{WasmInterpreter, WasmRuntime};
    use halo2_proofs::pairing::bn256::Fr as Fp;
    use specs::types::Value;

    #[test]
    fn test_fibonacci() {
        let textual_repr = r#"
        (module
            (memory $0 1)
            (export "memory" (memory $0))
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
        let compiled_module = compiler.compile(textual_repr).unwrap();
        let execution_log = compiler
            .run(&compiled_module, "fibonacci", vec![Value::I32(0)])
            .unwrap();
        run_test_circuit::<Fp>(compiled_module.tables, execution_log.tables).unwrap()
    }
}
