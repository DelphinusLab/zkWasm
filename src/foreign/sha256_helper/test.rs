#[cfg(test)]
pub(crate) mod tests {
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

    pub(crate) fn prepare_inputs() -> (Vec<u64>, Vec<u64>) {
        let msg = "abcdef";
        let mut msg: Vec<u64> = hex::decode(msg)
            .unwrap()
            .into_iter()
            .map(|v| v as u64)
            .collect();
        let mut private_inputs = vec![msg.len() as u64];
        private_inputs.append(&mut msg);

        let expected = "995da3cf545787d65f9ced52674e92ee8171c87c7a4008aa4349ec47d21609a7";
        let public_inputs = hex::decode(expected)
            .unwrap()
            .into_iter()
            .map(|v| v as u64)
            .collect();

        (public_inputs, private_inputs)
    }

    #[test]
    fn test_sha256() {
        let (public_inputs, private_inputs) = prepare_inputs();

        let mut wasm = vec![];

        let path = PathBuf::from("wasm/sha256.wasm");
        let mut f = File::open(path).unwrap();
        f.read_to_end(&mut wasm).unwrap();

        let compiler = WasmInterpreter::new();
        let mut env = HostEnv::new();
        register_sha256_foreign(&mut env);
        register_wasm_input_foreign(&mut env, public_inputs.clone(), private_inputs.clone());

        let imports = ImportsBuilder::new().with_resolver("env", &env);
        let compiled_module = compiler
            .compile(&wasm, &imports, &env.function_plugin_lookup)
            .unwrap();
        let execution_log = compiler
            .run(
                &mut env,
                &compiled_module,
                "sha256_digest",
                public_inputs.clone(),
                private_inputs,
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
