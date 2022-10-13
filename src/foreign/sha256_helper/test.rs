#[cfg(test)]
mod tests {
    use crate::{
        foreign::{
            sha256_helper::runtime::register_sha256_foreign,
            wasm_input_helper::runtime::register_wasm_input_foreign,
        },
        runtime::{host::HostEnv, WasmInterpreter, WasmRuntime},
        test::run_test_circuit,
    };

    use halo2_proofs::pairing::bn256::Fr as Fp;
    use std::{fs::File, io::Read, path::PathBuf};
    use wasmi::ImportsBuilder;

    #[test]
    fn test_sha256() {
        // hash 128bytes msg using sha256
        let public_inputs = vec![128];

        let mut wasm = vec![];

        let path = PathBuf::from("wasm/sha256_extern.wasm");
        let mut f = File::open(path).unwrap();
        f.read_to_end(&mut wasm).unwrap();

        let compiler = WasmInterpreter::new();
        let mut env = HostEnv::new();
        register_sha256_foreign(&mut env);
        register_wasm_input_foreign(&mut env, public_inputs.clone(), vec![]);

        let imports = ImportsBuilder::new().with_resolver("env", &env);
        let compiled_module = compiler
            .compile(&wasm, &imports, &env.function_plugin_lookup)
            .unwrap();
        let execution_log = compiler
            .run(
                &mut env,
                &compiled_module,
                "Hash_Calculate",
                public_inputs.clone(),
                vec![],
            )
            .unwrap();
        run_test_circuit::<Fp>(
            compiled_module.tables,
            execution_log.tables,
            public_inputs.into_iter().map(|v| Fp::from(v)).collect(),
        )
        .unwrap()
    }
}
