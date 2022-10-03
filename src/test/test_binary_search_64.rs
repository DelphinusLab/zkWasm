#[cfg(test)]
mod tests {
    use halo2_proofs::pairing::bn256::Fr as Fp;
    use specs::types::Value;
    use std::{collections::HashMap, fs::File, io::Read, path::PathBuf};
    use wasmi::{ImportsBuilder, NopExternals};

    use crate::{
        runtime::{WasmInterpreter, WasmRuntime},
        test::run_test_circuit,
    };

    #[test]
    fn test_binary_search_64() {
        let mut binary = vec![];

        let path = PathBuf::from("wasm/bsearch_64.wasm");
        let mut f = File::open(path).unwrap();
        f.read_to_end(&mut binary).unwrap();

        let compiler = WasmInterpreter::new();
        let compiled_module = compiler
            .compile(&binary, &ImportsBuilder::default(), HashMap::default())
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
}
