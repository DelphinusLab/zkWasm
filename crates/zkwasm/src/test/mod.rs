use crate::loader::slice::Slices;
use crate::loader::ZkWasmLoader;
use crate::runtime::host::default_env::DefaultHostEnvBuilder;
use crate::runtime::host::default_env::ExecutionArg;
use crate::runtime::host::HostEnvBuilder;
use crate::runtime::monitor::table_monitor::TableMonitor;

use anyhow::Result;
use halo2_proofs::pairing::bn256::Fr;
use specs::TraceBackend;
use wabt::wat2wasm_with_features;
use wabt::Features;

mod test_wasm_instructions;

mod spec;
mod test_rlp;
#[cfg(feature = "continuation")]
mod test_rlp_slice;
mod test_start;

/// Run the function and generate trace, then test circuit with mock prover.
pub fn test_circuit_with_env(
    k: u32,
    wasm: Vec<u8>,
    function_name: String,
    public_inputs: Vec<u64>,
    private_inputs: Vec<u64>,
) -> Result<()> {
    let module = ZkWasmLoader::parse_module(&wasm)?;

    let env = DefaultHostEnvBuilder.create_env(
        k,
        ExecutionArg {
            public_inputs,
            private_inputs,
            context_inputs: vec![],
        },
    );
    let mut monitor = TableMonitor::new(k, &vec![], TraceBackend::Memory, &env);
    let mut loader = ZkWasmLoader::new(k, env)?;
    loader.set_entry(function_name);

    let runner = loader.compile(&module, &mut monitor)?;

    let execution_result = loader.run(runner, &mut monitor)?;
    let instances: Vec<Fr> = execution_result.public_inputs_and_outputs();

    Slices::new(k, monitor.into_tables())?.mock_test_all(instances)?;

    Ok(())
}

/// Run test function and generate trace, then test circuit with mock prover. Only tests should
/// use this function.
fn test_circuit_noexternal(textual_repr: &str) -> Result<()> {
    use crate::circuits::config::MIN_K;

    let mut features = Features::new();
    features.enable_sign_extension();

    let wasm = wat2wasm_with_features(textual_repr, features).expect("failed to parse wat");

    test_circuit_with_env(MIN_K, wasm, "test".to_string(), vec![], vec![])?;

    Ok(())
}
