use crate::foreign::wasm_input_helper::runtime::register_wasm_input_foreign;
use crate::runtime::host::host_env::HostEnv;
use crate::runtime::wasmi_interpreter::Execution;
use crate::runtime::ExecutionResult;
use crate::runtime::WasmInterpreter;

use anyhow::Result;
use std::fs::{self};
use wasmi::ImportsBuilder;

fn build_test() -> Result<(ExecutionResult<wasmi::RuntimeValue>, Vec<u64>)> {
    let public_inputs = vec![2, 2];
    let private_inputs = vec![];

    let wasm = fs::read("wasm/rlp_simple.wasm").unwrap();
    let module = wasmi::Module::from_buffer(&wasm).expect("failed to load wasm");

    let mut env = HostEnv::new();
    register_wasm_input_foreign(&mut env, public_inputs.clone(), private_inputs);
    env.finalize();

    let imports = ImportsBuilder::new().with_resolver("env", &env);
    let compiler = WasmInterpreter::new();
    let compiled_module = compiler
        .compile(
            &module,
            &imports,
            &env.function_description_table(),
            "zkmain",
        )
        .unwrap();
    let execution_result = compiled_module.run(&mut env)?;

    Ok((execution_result, public_inputs))
}

mod tests {
    use super::*;
    use crate::circuits::config::set_zkwasm_k;
    use crate::circuits::ZkWasmCircuitBuilder;
    use crate::test::run_test_circuit;
    use halo2_proofs::pairing::bn256::Fr as Fp;
    use rusty_fork::rusty_fork_test;

    rusty_fork_test! {
        #[test]
        fn test_rlp_simple_mock() {
            set_zkwasm_k(20);

            let (execution_result, public_inputs) = build_test().unwrap();

            run_test_circuit(
                execution_result,
                public_inputs.into_iter().map(|v| Fp::from(v)).collect(),
            )
            .unwrap();
        }
    }

    rusty_fork_test! {
        #[test]
        fn test_rlp_bench() {
            set_zkwasm_k(20);

            let (execution_result, public_inputs) = build_test().unwrap();

            let builder = ZkWasmCircuitBuilder {
                fid_of_entry: execution_result.fid_of_entry,
                tables: execution_result.tables,
            };

            builder.bench(public_inputs.into_iter().map(|v| Fp::from(v)).collect())
        }
    }
}
