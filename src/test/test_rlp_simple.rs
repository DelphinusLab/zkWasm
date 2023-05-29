use crate::foreign::wasm_input_helper::runtime::register_wasm_input_foreign;
use crate::runtime::host::host_env::HostEnv;
use crate::runtime::ExecutionResult;

use super::compile_then_execute_wasm;
use anyhow::Result;

fn build_test() -> Result<ExecutionResult<wasmi::RuntimeValue>> {
    let public_inputs = vec![2, 2];
    let private_inputs = vec![];

    let wasm = std::fs::read("wasm/rlp_simple.wasm").unwrap();

    let mut env = HostEnv::new();
    let wasm_runtime_io =
        register_wasm_input_foreign(&mut env, public_inputs.clone(), private_inputs);
    env.finalize();

    let trace = compile_then_execute_wasm(env, wasm_runtime_io, wasm, "zkmain")?;

    Ok(trace)
}

mod tests {
    use super::*;
    use crate::circuits::config::set_zkwasm_k;
    use crate::circuits::ZkWasmCircuitBuilder;
    use crate::test::test_circuit_mock;
    use halo2_proofs::pairing::bn256::Fr as Fp;
    use rusty_fork::rusty_fork_test;

    rusty_fork_test! {
        #[test]
        fn test_rlp_simple_mock() {
            set_zkwasm_k(20);

            let trace = build_test().unwrap();

            test_circuit_mock::<Fp>(trace).unwrap();
        }
    }

    rusty_fork_test! {
        #[test]
        fn test_rlp_bench() {
            set_zkwasm_k(20);

            let execution_result = build_test().unwrap();

            let builder = ZkWasmCircuitBuilder {
                tables: execution_result.tables,
                public_inputs_and_outputs:execution_result.public_inputs_and_outputs,
            };

            builder.bench()
        }
    }
}
