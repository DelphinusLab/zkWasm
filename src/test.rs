pub mod test_circuit;
pub mod test_circuit_builder;

#[cfg(test)]
pub mod tests {
    use super::test_circuit_builder::run_test_circuit;
    use crate::runtime::{WasmInterpreter, WasmRuntime};
    use halo2_proofs::pairing::bn256::Fr as Fp;
    use specs::types::Value;

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
        let compiled_module = compiler.compile(textual_repr).unwrap();
        let execution_log = compiler
            .run(&compiled_module, "fibonacci", vec![Value::I32(15)])
            .unwrap();
        run_test_circuit::<Fp>(compiled_module.tables, execution_log.tables).unwrap()
    }

    /*
     * int arr[2] = {1,2};
     * int memory_rw() {
     *   arr[0] = arr[0] + arr[1];
     *   return arr[0];
     * }
     */
    #[test]
    fn test_memory_rw() {
        let textual_repr = r#"
           (module
            (table 0 anyfunc)
            (memory $0 1)
            (data (i32.const 12) "\01\00\00\00\02\00\00\00")
            (export "memory" (memory $0))
            (export "memory_rw" (func $memory_rw))
            (func $memory_rw (; 0 ;) (result i32)
             (local $0 i32)
             (i32.store offset=12
              (i32.const 0)
              (tee_local $0
               (i32.add
                (i32.load offset=12
                 (i32.const 0)
                )
                (i32.load offset=16
                 (i32.const 0)
                )
               )
              )
             )
             (get_local $0)
            )
           )
        "#;

        let compiler = WasmInterpreter::new();
        let compiled_module = compiler.compile(textual_repr).unwrap();
        let execution_log = compiler.run(&compiled_module, "memory_rw", vec![]).unwrap();
        run_test_circuit::<Fp>(compiled_module.tables, execution_log.tables).unwrap()
    }
}
