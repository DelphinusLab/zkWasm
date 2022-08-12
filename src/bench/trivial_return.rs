#[cfg(test)]
mod tests {
    use specs::types::Value;
    use wasmi::{ImportsBuilder, NopExternals};

    use crate::{
        circuits::ZkWasmCircuitBuilder,
        runtime::{WasmInterpreter, WasmRuntime},
    };

    #[test]
    fn test_trivial_return_bench() {
        let textual_repr = r#"
        (module
            (func (export "test")
              return
            )
           )
        "#;

        let binary = wabt::wat2wasm(&textual_repr).expect("failed to parse wat");

        let compiler = WasmInterpreter::new();
        let compiled_module = compiler
            .compile(&binary, &ImportsBuilder::default())
            .unwrap();
        let execution_log = compiler
            .run(
                &mut NopExternals,
                &compiled_module,
                "test",
                vec![],
            )
            .unwrap();

        let builder = ZkWasmCircuitBuilder {
            compile_tables: compiled_module.tables,
            execution_tables: execution_log.tables,
        };

        builder.bench()
    }
}
