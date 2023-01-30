#[cfg(test)]
pub(crate) mod tests {
    use crate::{
        foreign::wasm_input_helper::runtime::register_wasm_input_foreign,
        runtime::{host::HostEnv, wasmi_interpreter::Execution, WasmInterpreter, WasmRuntime},
        test::run_test_circuit,
    };

    use crate::circuits::config::set_zkwasm_k;
    use halo2_proofs::pairing::bn256::Fr as Fp;
    use rusty_fork::rusty_fork_test;
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

        let expected = "acd0c377fe36d5b209125185bc3ac41155ed1bf7103ef9f0c2aff4320460b6df";
        let public_inputs = hex::decode(expected)
            .unwrap()
            .into_iter()
            .map(|v| v as u64)
            .collect();

        (public_inputs, private_inputs)
    }

    /*
     *  FORK the test since it modifies global variable zkwasm_k.
     */
    rusty_fork_test! {
        #[test]
        fn test_keccak() {
            let (public_inputs, private_inputs) = prepare_inputs();

            let mut wasm = vec![];

            let path = PathBuf::from("wasm/keccak.wasm");
            let mut f = File::open(path).unwrap();
            f.read_to_end(&mut wasm).unwrap();

            let compiler = WasmInterpreter::new();
            let mut env = HostEnv::new();
            register_wasm_input_foreign(&mut env, public_inputs.clone(), private_inputs.clone());

            let imports = ImportsBuilder::new().with_resolver("env", &env);
            let compiled_module = compiler
                .compile(&wasm, &imports, &env.function_plugin_lookup)
                .unwrap();
            let execution_result = compiled_module
                .run(
                    &mut env,
                    "keccak_digest",
                )
                .unwrap();
            set_zkwasm_k(19);
            run_test_circuit::<Fp>(
                execution_result.tables,
                public_inputs.into_iter().map(|v| Fp::from(v)).collect(),
            )
            .unwrap()
        }
    }
}
