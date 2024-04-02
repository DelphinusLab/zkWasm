use crate::circuits::config::MIN_K;
use crate::foreign::context::ContextOutput;
use crate::loader::ZkWasmLoader;
use crate::runtime::host::default_env::DefaultHostEnvBuilder;
use crate::runtime::host::default_env::ExecutionArg;

use anyhow::Result;
use halo2_proofs::pairing::bn256::Bn256;
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
    wasm: Vec<u8>,
    function_name: String,
    public_inputs: Vec<u64>,
    private_inputs: Vec<u64>,
) -> Result<()> {
    let mut loader = ZkWasmLoader::<Bn256, _, DefaultHostEnvBuilder>::new(MIN_K, wasm, vec![])?;
    loader.set_entry(function_name);
    let execution_result = loader.run(
        ExecutionArg {
            public_inputs,
            private_inputs,
            context_inputs: vec![],
            context_outputs: ContextOutput::default(),
        },
        (),
        false,
        TraceBackend::Memory,
    )?;
    let instances = execution_result.public_inputs_and_outputs();
    loader
        .slice(execution_result)
        .mock_test_all(MIN_K, instances)?;

    Ok(())
}

/// Run test function and generate trace, then test circuit with mock prover. Only tests should
/// use this function.
fn test_circuit_noexternal(textual_repr: &str) -> Result<()> {
    let mut features = Features::new();
    features.enable_sign_extension();

    let wasm = wat2wasm_with_features(&textual_repr, features).expect("failed to parse wat");

    test_circuit_with_env(wasm, "test".to_string(), vec![], vec![])?;

    Ok(())
}
