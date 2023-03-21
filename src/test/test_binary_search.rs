use crate::foreign::wasm_input_helper::runtime::register_wasm_input_foreign;
use crate::runtime::host::host_env::HostEnv;
use crate::runtime::ExecutionResult;

use anyhow::Result;
use halo2_proofs::pairing::bn256::Fr as Fp;
use std::fs::{self};

use super::test_circuit_with_env;

fn build_test() -> Result<(ExecutionResult<wasmi::RuntimeValue>, Vec<u64>)> {
    let public_inputs = vec![3];

    let wasm = fs::read("wasm/bsearch_64.wasm").unwrap();

    let mut env = HostEnv::new();
    register_wasm_input_foreign(&mut env, public_inputs.clone(), vec![]);
    env.finalize();

    let execution_result = test_circuit_with_env(
        env,
        wasm,
        "bsearch",
        public_inputs.iter().map(|v| Fp::from(*v)).collect(),
    )?;

    Ok((execution_result, public_inputs))
}

mod tests {
    use super::*;
    use crate::circuits::ZkWasmCircuitBuilder;
    use crate::test::run_test_circuit;
    use halo2_proofs::pairing::bn256::Fr as Fp;

    #[test]
    fn test_binary_search_mock() {
        let (result, public_inputs) = build_test().unwrap();

        run_test_circuit(
            result,
            public_inputs.into_iter().map(|v| Fp::from(v)).collect(),
        )
        .unwrap();
    }

    #[test]
    fn test_binary_search_full() {
        let (execution_result, public_inputs) = build_test().unwrap();

        let builder = ZkWasmCircuitBuilder {
            tables: execution_result.tables,
        };

        builder.bench(public_inputs.into_iter().map(|v| Fp::from(v)).collect())
    }
}
