use crate::circuits::config::zkwasm_k;
use crate::circuits::utils::table_entry::MemoryWritingTable;
use crate::circuits::TestCircuit;
use crate::profile::Profiler;
use crate::runtime::host::host_env::HostEnv;
use crate::runtime::wasmi_interpreter::Execution;
use crate::runtime::wasmi_interpreter::WasmRuntimeIO;
use crate::runtime::ExecutionResult;
use crate::runtime::WasmInterpreter;

use anyhow::Result;
use halo2_proofs::arithmetic::FieldExt;
use halo2_proofs::dev::MockProver;
use halo2_proofs::pairing::bn256::Fr;
use wasmi::ImportsBuilder;
use wasmi::RuntimeValue;

#[cfg(test)]
mod test_wasm_instructions;

mod spec;
mod test_rlp;
mod test_rlp_simple;
mod test_start;
mod test_uniform_verifier;

/// Create circuit with trace and run mock test.
fn test_circuit_mock<F: FieldExt>(
    execution_result: ExecutionResult<wasmi::RuntimeValue>,
) -> Result<()> {
    let instance = {
        let mut v: Vec<F> = vec![];

        v.append(
            &mut execution_result
                .public_inputs_and_outputs
                .iter()
                .map(|v| (*v).into())
                .collect(),
        );

        v
    };

    execution_result.tables.write_json(None);
    let memory_writing_table: MemoryWritingTable = execution_result
        .tables
        .execution_tables
        .mtable
        .clone()
        .into();
    memory_writing_table.write_json(None);

    execution_result.tables.profile_tables();

    let circuit = TestCircuit::new(execution_result.tables);
    let prover = MockProver::run(zkwasm_k(), &circuit, vec![instance])?;
    assert_eq!(prover.verify(), Ok(()));

    Ok(())
}

/// Run function and generate trace.
fn compile_then_execute_wasm(
    mut env: HostEnv,
    wasm_runtime_io: WasmRuntimeIO,
    wasm: Vec<u8>,
    function_name: &str,
) -> Result<ExecutionResult<RuntimeValue>> {
    let module = wasmi::Module::from_buffer(&wasm).expect("failed to load wasm");

    let imports = ImportsBuilder::new().with_resolver("env", &env);

    let compiled_module = WasmInterpreter::compile(
        &module,
        &imports,
        &env.function_description_table(),
        function_name,
        &vec![],
    )
    .unwrap();

    let execution_result = compiled_module.run(&mut env, wasm_runtime_io)?;

    Ok(execution_result)
}

/// Run the function and generate trace, then test circuit with mock prover.
pub fn test_circuit_with_env(
    env: HostEnv,
    wasm_runtime_io: WasmRuntimeIO,
    wasm: Vec<u8>,
    function_name: &str,
) -> Result<ExecutionResult<RuntimeValue>> {
    let trace = compile_then_execute_wasm(env, wasm_runtime_io, wasm, function_name)?;
    test_circuit_mock::<Fr>(trace.clone())?;

    Ok(trace)
}

/// Run test function and generate trace, then test circuit with mock prover. Only tests should
/// use this function.
fn test_circuit_noexternal(textual_repr: &str) -> Result<()> {
    let wasm = wabt::wat2wasm(&textual_repr).expect("failed to parse wat");

    let mut env = HostEnv::new();
    env.finalize();

    test_circuit_with_env(env, WasmRuntimeIO::empty(), wasm, "test")?;

    Ok(())
}
