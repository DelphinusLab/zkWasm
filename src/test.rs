pub mod test_circuit_builder;

#[cfg(test)]
pub mod tests {
    use std::{fs::File, io::Read, path::PathBuf};

    use super::test_circuit_builder::run_test_circuit;
    use crate::runtime::{WasmInterpreter, WasmRuntime};
    use halo2_proofs::pairing::bn256::Fr as Fp;
    use specs::types::Value;
    use wasmi::{ImportsBuilder, NopExternals};

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

        let wasm = wabt::wat2wasm(textual_repr).unwrap();

        let compiler = WasmInterpreter::new();
        let compiled_module = compiler.compile(&wasm, &ImportsBuilder::default()).unwrap();
        let execution_log = compiler
            .run(&mut NopExternals, &compiled_module, "memory_rw", vec![])
            .unwrap();
        run_test_circuit::<Fp>(compiled_module.tables, execution_log.tables).unwrap()
    }

    /*
      unsigned int bsearch(unsigned int v) {

        unsigned int array[5] = {1,2,3,4,5};
        unsigned int l= 0;
        unsigned int r = 4;
        unsigned int mid;

        while (l <= r) {

                mid = (l + r) / 2;
                if (array[mid] < v) {
                        l = mid + 1;
                } else if (array[mid] > v) {
                        r = mid - 1;
                } else {
                        return mid;
                }
        }

        return 5;
      }
    */
    #[test]
    fn test_binary_search() {
        let mut binary = vec![];

        let path = PathBuf::from("wasm/bsearch.wasm");
        let mut f = File::open(path).unwrap();
        f.read_to_end(&mut binary).unwrap();

        let compiler = WasmInterpreter::new();
        let compiled_module = compiler
            .compile(&binary, &ImportsBuilder::default())
            .unwrap();
        let execution_log = compiler
            .run(
                &mut NopExternals,
                &compiled_module,
                "bsearch",
                vec![Value::I32(3)],
            )
            .unwrap();
        run_test_circuit::<Fp>(compiled_module.tables, execution_log.tables).unwrap()
    }

    #[test]
    fn test_binary_search_64() {
        let mut binary = vec![];

        let path = PathBuf::from("wasm/bsearch_64.wasm");
        let mut f = File::open(path).unwrap();
        f.read_to_end(&mut binary).unwrap();

        let compiler = WasmInterpreter::new();
        let compiled_module = compiler
            .compile(&binary, &ImportsBuilder::default())
            .unwrap();
        let execution_log = compiler
            .run(
                &mut NopExternals,
                &compiled_module,
                "bsearch",
                vec![Value::I64(3)],
            )
            .unwrap();
        run_test_circuit::<Fp>(compiled_module.tables, execution_log.tables).unwrap()
    }

    #[test]
    fn test_sha256() {
        let mut binary = vec![];

        let path = PathBuf::from("wasm/sha256.wasm");
        let mut f = File::open(path).unwrap();
        f.read_to_end(&mut binary).unwrap();

        let compiler = WasmInterpreter::new();
        let compiled_module = compiler
            .compile(&binary, &ImportsBuilder::default())
            .unwrap();
        let execution_log = compiler
            .run(
                &mut NopExternals,
                &compiled_module,
                "Hash_Calculate",
                vec![Value::I32(1024)],
            )
            .unwrap();
        run_test_circuit::<Fp>(compiled_module.tables, execution_log.tables).unwrap()
    }
}
