use crate::{
    circuits::{config::zkwasm_k, TestCircuit},
    profile::Profile,
    runtime::{
        host::host_env::HostEnv, wasmi_interpreter::Execution, ExecutionResult, WasmInterpreter,
    },
};

use anyhow::Result;
use halo2_proofs::{arithmetic::FieldExt, dev::MockProver, pairing::bn256::Fr as Fp};
use specs::Tables;
use wasmi::{ImportsBuilder, RuntimeValue};

mod spec;
mod test_binary_search;
mod test_fibonacci;
mod test_rlp;
mod test_sha256;
mod test_start;
mod test_wasm_instructions;

fn test_circuit(
    mut env: HostEnv,
    wasm: Vec<u8>,
    function_name: &str,
    public_inputs: Vec<Fp>,
) -> Result<ExecutionResult<RuntimeValue>> {
    let module = wasmi::Module::from_buffer(&wasm).expect("failed to load wasm");

    let imports = ImportsBuilder::new().with_resolver("env", &env);

    let compiler = WasmInterpreter::new();
    let compiled_module = compiler
        .compile(
            &module,
            &imports,
            &env.function_description_table(),
            function_name,
        )
        .unwrap();
    let execution_result = compiled_module.run(&mut env)?;

    run_test_circuit::<Fp>(execution_result.tables.clone(), public_inputs)?;

    Ok(execution_result)
}

fn test_circuit_noexternal_function(textual_repr: &str, function_name: &str) -> Result<()> {
    let wasm = wabt::wat2wasm(&textual_repr).expect("failed to parse wat");

    let mut env = HostEnv::new();
    env.finalize();

    test_circuit(env, wasm, function_name, vec![]).unwrap();

    Ok(())
}

pub fn test_circuit_noexternal(textual_repr: &str) -> Result<()> {
    test_circuit_noexternal_function(textual_repr, "test").unwrap();

    Ok(())
}

pub fn test_circuit_with_env(
    env: HostEnv,
    wasm: Vec<u8>,
    function_name: &str,
    public_inputs: Vec<Fp>,
) -> Result<ExecutionResult<RuntimeValue>> {
    test_circuit(env, wasm, function_name, public_inputs)
}

pub fn run_test_circuit<F: FieldExt>(tables: Tables, public_inputs: Vec<F>) -> Result<()> {
    tables.write_json(None);

    tables.execution_tables.etable.profile_instruction();

    let circuit = TestCircuit::<F>::new(tables);

    let prover = MockProver::run(zkwasm_k(), &circuit, vec![public_inputs])?;
    assert_eq!(prover.verify(), Ok(()));

    Ok(())
}
