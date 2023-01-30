#[cfg(test)]
mod tests {
    use crate::{
        circuits::ZkWasmCircuitBuilder,
        foreign::{
            sha256_helper::{runtime::register_sha256_foreign, test::tests::prepare_inputs},
            wasm_input_helper::runtime::register_wasm_input_foreign,
        },
        runtime::{host::HostEnv, wasmi_interpreter::Execution, WasmInterpreter, WasmRuntime},
    };
    use halo2_proofs::pairing::bn256::Fr as Fp;
    use std::{fs::File, io::Read, path::PathBuf};
    use wasmi::ImportsBuilder;

    #[test]
    fn test_sha256() {
        let (public_inputs, private_inputs) = prepare_inputs();

        let mut binary = vec![];

        let path = PathBuf::from("wasm/sha256.wasm");
        let mut f = File::open(path).unwrap();
        f.read_to_end(&mut binary).unwrap();

        let compiler = WasmInterpreter::new();

        let mut env = HostEnv::new();
        register_wasm_input_foreign(&mut env, public_inputs.clone(), private_inputs.clone());
        register_sha256_foreign(&mut env);
        let imports = ImportsBuilder::new().with_resolver("env", &env);

        let compiled_module = compiler
            .compile(&binary, &imports, &env.function_plugin_lookup)
            .unwrap();
        let execution_result = compiled_module.run(&mut env, "sha256_digest").unwrap();

        let builder = ZkWasmCircuitBuilder {
            tables: execution_result.tables,
        };

        builder.bench(public_inputs.into_iter().map(|v| Fp::from(v)).collect())
    }
}
