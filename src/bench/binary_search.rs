#[cfg(test)]
mod tests {
    use std::{fs::File, io::Read, path::PathBuf};

    use specs::types::Value;
    use wasmi::{ImportsBuilder, NopExternals};

    use crate::{
        circuits::ZkWasmCircuitBuilder,
        runtime::{WasmInterpreter, WasmRuntime},
    };

    #[test]
    fn test_binary_search_10000() {
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

        let builder = ZkWasmCircuitBuilder {
            compile_tables: compiled_module.tables,
            execution_tables: execution_log.tables,
        };

        builder.bench()
    }
}
