use crate::foreign::wasm_input_helper::runtime::register_wasm_input_foreign;
use crate::runtime::host::host_env::HostEnv;
use crate::runtime::ExecutionResult;

use anyhow::Result;
use std::fs::{self};

use super::compile_then_execute_wasm;

fn build_test() -> Result<ExecutionResult<wasmi::RuntimeValue>> {
    let public_inputs = vec![3];

    let wasm = fs::read("wasm/bsearch_64.wasm").unwrap();

    let mut env = HostEnv::new();
    let wasm_runtime_io = register_wasm_input_foreign(&mut env, public_inputs, vec![]);
    env.finalize();

    compile_then_execute_wasm(env, wasm_runtime_io, wasm, "bsearch")
}

mod tests {
    use super::*;
    use crate::circuits::ZkWasmCircuitBuilder;
    use crate::test::test_circuit_mock;
    use halo2_proofs::pairing::bn256::Fr as Fp;

    #[test]
    fn test_binary_search_mock() {
        let trace = build_test().unwrap();

        test_circuit_mock::<Fp>(trace).unwrap();
    }

    #[test]
    fn test_binary_search_full() {
        let execution_result = build_test().unwrap();

        let builder = ZkWasmCircuitBuilder {
            tables: execution_result.tables,
            public_inputs_and_outputs: execution_result.public_inputs_and_outputs,
        };

        builder.bench()
    }
}
