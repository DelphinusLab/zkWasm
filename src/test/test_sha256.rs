#[cfg(test)]
mod tests {
    use crate::{
        circuits::ZkWasmCircuitBuilder,
        foreign::{
            sha256_helper::{runtime::register_sha256_foreign, test::tests::prepare_inputs},
            wasm_input_helper::runtime::register_wasm_input_foreign,
        },
        runtime::host::host_env::HostEnv,
        test::test_circuit_with_env,
    };
    use halo2_proofs::pairing::bn256::Fr as Fp;
    use std::fs::{self};

    #[test]
    fn test_sha256_full() {
        let (public_inputs, private_inputs) = prepare_inputs();

        let wasm = fs::read("wasm/sha256.wasm").unwrap();

        let mut env = HostEnv::new();
        register_wasm_input_foreign(&mut env, public_inputs.clone(), private_inputs.clone());
        register_sha256_foreign(&mut env);
        env.finalize();

        let public_inputs: Vec<Fp> = public_inputs.into_iter().map(|v| Fp::from(v)).collect();

        let execution_result =
            test_circuit_with_env(env, wasm, "sha256_digest", public_inputs.clone()).unwrap();

        let builder = ZkWasmCircuitBuilder {
            fid_of_entry: execution_result.fid_of_entry,
            tables: execution_result.tables,
        };

        builder.bench(public_inputs)
    }
}
