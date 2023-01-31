#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use wasmi::{ImportsBuilder, NopExternals};

    use crate::{
        circuits::ZkWasmCircuitBuilder,
        runtime::{wasmi_interpreter::Execution, WasmInterpreter, WasmRuntime},
    };

    #[test]
    fn test_return_full() {
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
            .compile(&binary, &ImportsBuilder::default(), &HashMap::new())
            .unwrap();
        let execution_result = compiled_module.run(&mut NopExternals, "test").unwrap();

        let builder = ZkWasmCircuitBuilder {
            tables: execution_result.tables,
        };

        builder.bench(vec![])
    }
}
