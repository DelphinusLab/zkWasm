#[cfg(test)]
pub(crate) mod tests {
    use std::fs;

    use crate::{
        foreign::{
            require_helper::register_require_foreign,
            sha256_helper::runtime::register_sha256_foreign,
            wasm_input_helper::runtime::register_wasm_input_foreign,
        },
        runtime::host::host_env::HostEnv,
        test::test_circuit_with_env,
    };

    use halo2_proofs::pairing::bn256::Fr as Fp;

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

        let wasm = fs::read("wasm/sha256.wasm").unwrap();

        let mut env = HostEnv::new();
        register_sha256_foreign(&mut env);
        register_wasm_input_foreign(&mut env, public_inputs.clone(), private_inputs.clone());
        env.finalize();

        test_circuit_with_env(
            env,
            wasm,
            "sha256_digest",
            public_inputs.into_iter().map(|v| Fp::from(v)).collect(),
        )
        .unwrap();
    }

    #[test]
    fn test_sha256_v2() {
        let wasm = fs::read("wasm/sha256_v2.wasm").unwrap();

        let mut env = HostEnv::new();
        register_sha256_foreign(&mut env);
        register_wasm_input_foreign(&mut env, vec![], vec![]);
        register_require_foreign(&mut env);
        env.finalize();

        test_circuit_with_env(env, wasm, "zkmain", vec![]).unwrap();
    }
}
