#[cfg(test)]
pub(crate) mod tests {
    use crate::foreign::wasm_input_helper::runtime::register_wasm_input_foreign;
    use crate::runtime::host::host_env::HostEnv;
    use crate::test::test_circuit_with_env;

    use crate::circuits::config::set_zkwasm_k;
    use halo2_proofs::pairing::bn256::Fr as Fp;
    use rusty_fork::rusty_fork_test;
    use std::fs;

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
            set_zkwasm_k(19);

            let (public_inputs, private_inputs) = prepare_inputs();

            let wasm = fs::read("wasm/keccak.wasm").unwrap();

            let mut env = HostEnv::new();
            register_wasm_input_foreign(&mut env, public_inputs.clone(), private_inputs.clone());
            env.finalize();


            test_circuit_with_env(env, wasm, "keccak_digest", public_inputs.into_iter().map(|v| Fp::from(v)).collect()).unwrap();
        }
    }
}
