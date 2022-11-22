#[cfg(test)]
mod tests {
    use std::collections::HashMap;

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

        let compiler = WasmInterpreter::new(HashMap::new());
        let compiled_module = compiler
            .compile(&binary, &ImportsBuilder::default())
            .unwrap();
        let _ = compiler
            .run(&mut NopExternals, &compiled_module, "test", vec![], vec![])
            .unwrap();

        let builder = ZkWasmCircuitBuilder::from_wasm_runtime(&compiler);

        builder.bench(vec![])
    }
}
