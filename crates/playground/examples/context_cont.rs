use std::fs::File;
use std::fs::{self};
use std::io::Write;
use std::path::PathBuf;

use anyhow::Result;
use delphinus_zkwasm::loader::ZkWasmLoader;
use pairing_bn256::bn256::Bn256;

const TMP_PATH: &str = "context_cont.context.tmp";

fn main() -> Result<()> {
    let wasm = std::fs::read("wasm/context_cont.wasm")?;
    let context_in = vec![2, 1];
    let mut fd = File::create("./context_cont.context.tmp")?;
    context_in.into_iter().for_each(|v| {
        let mut buf = u64::to_le_bytes(v);

        fd.write_all(&mut buf).unwrap();
    });

    let loader = ZkWasmLoader::<Bn256>::new(18, wasm, vec![])?;

    let (circuit, instances) =
        loader.circuit_with_witness(vec![], vec![0], Some(PathBuf::from(TMP_PATH)), None)?;

    fs::remove_file(TMP_PATH)?;

    loader.mock_test(&circuit, &instances)
}
