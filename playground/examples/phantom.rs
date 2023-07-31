use anyhow::Result;
use delphinus_zkwasm::loader::ZkWasmLoader;
use pairing_bn256::bn256::Bn256;

fn main() -> Result<()> {
    let wasm = std::fs::read("wasm/phantom.wasm")?;

    let loader = ZkWasmLoader::<Bn256>::new(
        18,
        wasm,
        vec!["search"].into_iter().map(|s| s.to_owned()).collect(),
    )?;

    let (circuit, instances) = loader.circuit_with_witness(vec![], vec![], None, None)?;
    loader.mock_test(&circuit, &instances)
}
