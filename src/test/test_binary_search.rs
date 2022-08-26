#[cfg(test)]
mod tests {
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

    use halo2_proofs::pairing::bn256::Fr as Fp;
    use specs::types::Value;
    use std::{fs::File, path::PathBuf, io::Read};
    use wasmi::{ImportsBuilder, NopExternals};

    use crate::{runtime::{WasmInterpreter, WasmRuntime}, test::run_test_circuit};

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
}
