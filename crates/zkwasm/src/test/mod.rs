use crate::circuits::MIN_K;
use crate::loader::slice::Slices;
use crate::loader::ZkWasmLoader;
use crate::runtime::host::default_env::DefaultHostEnvBuilder;
use crate::runtime::host::default_env::ExecutionArg;
use crate::runtime::host::HostEnvBuilder;
use crate::runtime::monitor::table_monitor::TableMonitor;

use anyhow::Result;
use halo2_proofs::pairing::bn256::Fr;
use specs::slice_backend::InMemoryBackendBuilder;
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
    public_inputs: Vec<u64>,
    private_inputs: Vec<u64>,
) -> Result<()> {
    let module = ZkWasmLoader::parse_module(&wasm)?;
    let env_builder = DefaultHostEnvBuilder::new(k);

    let env = env_builder.create_env(ExecutionArg {
        public_inputs,
        private_inputs,
        context_inputs: vec![],
        indexed_witness: Default::default(),
        tree_db: None,
    });
    let mut monitor = TableMonitor::new(
        k,
        InMemoryBackendBuilder,
        env_builder.create_flush_strategy(),
        &vec![],
        &env,
    );
    let loader = ZkWasmLoader::new(k, env)?;

    let runner = loader.compile(&module, &mut monitor)?;

    let execution_result = loader.run(runner, &mut monitor)?;
    let instances: Vec<Fr> = execution_result.public_inputs_and_outputs();

    Slices::new(k, monitor.into_tables(), None)?.mock_test_all(instances)?;

    Ok(())
}

/// Run test function and generate trace, then test circuit with mock prover. Only tests should
/// use this function.
fn test_instruction(textual_repr: &str) -> Result<()> {
    let mut features = Features::new();
    features.enable_sign_extension();

    let wasm = wat2wasm_with_features(textual_repr, features).expect("failed to parse wat");

    test_circuit_with_env(MIN_K, wasm, vec![], vec![])?;

    Ok(())
}
