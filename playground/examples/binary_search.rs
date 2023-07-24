use anyhow::Result;
use delphinus_zkwasm::loader::ZkWasmLoader;
use pairing_bn256::bn256::Bn256;

fn main() -> Result<()> {
    let wasm = std::fs::read("wasm/binary_search.wasm")?;

    let loader = ZkWasmLoader::<Bn256>::new(18, wasm, vec![])?;

    let (circuit, instances) = loader.circuit_with_witness(vec![0], vec![])?;
    loader.mock_test(&circuit, &instances)
}
