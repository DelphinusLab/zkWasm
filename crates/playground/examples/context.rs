use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

use anyhow::Result;
use delphinus_zkwasm::loader::ZkWasmLoader;
use pairing_bn256::bn256::Bn256;

const CONTEXT_IN_PATH: &str = "context_in.context.tmp";
const CONTEXT_OUT_PATH: &str = "context_out.context.tmp";

fn main() -> Result<()> {
    let wasm = std::fs::read("wasm/context_cont.wasm")?;
    let context_in = vec![2, 1];
    let mut fd = File::create(CONTEXT_IN_PATH)?;
    context_in.into_iter().for_each(|v| {
        let mut buf = u64::to_le_bytes(v);

        fd.write_all(&mut buf).unwrap();
    });

    let loader = ZkWasmLoader::<Bn256>::new(18, wasm, vec![])?;

    let (circuit, instances) = loader.circuit_with_witness(
        vec![],
        vec![],
        Some(PathBuf::from(CONTEXT_IN_PATH)),
        Some(PathBuf::from(CONTEXT_OUT_PATH)),
    )?;
    loader.mock_test(&circuit, &instances)?;

    let (circuit, instances) = loader.circuit_with_witness(
        vec![],
        vec![],
        Some(PathBuf::from(CONTEXT_OUT_PATH)),
        Some(PathBuf::from(CONTEXT_OUT_PATH)),
    )?;
    loader.mock_test(&circuit, &instances)?;

    std::fs::remove_file(CONTEXT_IN_PATH)?;
    std::fs::remove_file(CONTEXT_OUT_PATH)?;

    Ok(())
}
