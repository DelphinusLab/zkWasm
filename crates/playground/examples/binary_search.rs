use anyhow::Result;
use delphinus_zkwasm::foreign::context::ContextOutput;
use delphinus_zkwasm::loader::TraceBackend;
use delphinus_zkwasm::loader::ZkWasmLoader;
use delphinus_zkwasm::runtime::host::default_env::DefaultHostEnvBuilder;
use delphinus_zkwasm::runtime::host::default_env::ExecutionArg;
use pairing_bn256::bn256::Bn256;

fn main() -> Result<()> {
    let wasm = std::fs::read("wasm/binary_search.wasm")?;

    let loader = ZkWasmLoader::<Bn256, ExecutionArg, DefaultHostEnvBuilder>::new(18, wasm, vec![])?;
    let result = loader.run(
        ExecutionArg {
            public_inputs: vec![0],
            private_inputs: vec![],
            context_inputs: vec![],
            context_outputs: ContextOutput::default(),
        },
        (),
        false,
        TraceBackend::Memory,
    )?;
    let instances = result.public_inputs_and_outputs();

    let slices = loader.slice(result);
    slices.mock_test_all(18, instances)?;

    Ok(())
}
