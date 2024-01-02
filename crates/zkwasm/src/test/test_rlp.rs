use std::sync::Arc;
use std::sync::Mutex;

use crate::circuits::TestCircuit;
use crate::loader::ZkWasmLoader;
use crate::runtime::host::default_env::DefaultHostEnvBuilder;
use crate::runtime::host::default_env::ExecutionArg;

use anyhow::Result;
use halo2_proofs::pairing::bn256::Bn256;
use halo2_proofs::pairing::bn256::Fr;

fn build_circuit() -> Result<(
    ZkWasmLoader<Bn256, ExecutionArg, DefaultHostEnvBuilder>,
    TestCircuit<Fr>,
    Vec<Fr>,
)> {
    let public_inputs = vec![133];
    let private_inputs: Vec<u64> = vec![
        14625441452057167097,
        441,
        0,
        0,
        144115188084244480,
        17592186044416,
        0,
        0,
        2,
        0,
        281474976710656,
        72057594037928224,
        0,
        144115188075855872,
        4398046511104,
        2048,
        0,
        288230376151711744,
        562949953421312,
        36033195065475072,
        0,
        1152921504606846992,
        0,
        72057594037927936,
        0,
        0,
        72057594037927936,
        274877906944,
        0,
        8192,
        0,
        0,
        0,
        142172368092004352,
        10663670667014018268,
        15598333267600830878,
        4825637194728734969,
        11537926770794296992,
        8941585237026987872,
        1060144843738714138,
        15286290987074524363,
        41041,
        0,
        0,
        0,
        549784760702,
        0,
        0,
        13839280179932823552,
        9466528,
        0,
        1245741926200423424,
        9993052845762533317,
        643603743268,
        0,
        0,
        0,
        687194767360,
        0,
        0,
        0,
        274894684160,
        0,
        17752714368831347629,
        14734568103978781184,
        16340025600,
        0,
        0,
        0,
        17179869184,
        0,
        0,
        13839280179932823552,
        9466528,
        0,
        0,
        13839280179932823552,
        9466528,
        0,
        0,
        13839280179932823552,
        9466528,
        0,
        0,
        13983395368008679424,
        180934170288,
        0,
        0,
        0,
        216736848758702080,
        0,
        0,
        0,
        10708425217887174656,
        8187143887307480351,
        70325280878010241,
        117203507575396024,
        11486502108844260361,
        13539931197926996738,
        18161434576524511916,
        11561024771253616253,
        0,
        0,
        0,
        12789659991778787328,
        160,
        0,
        0,
        0,
        40960,
        0,
        0,
        15880255236061790208,
        17950538412901046486,
        8547692942764276983,
        8509190860294355049,
        5730928406529570843,
        18210150271972058323,
        3994395479395232905,
        6563862530498629762,
        688805136118,
        0,
        0,
        13839280179932823552,
        175921869910688,
        0,
        0,
        0,
        45231150997700608,
        0,
        0,
        0,
        43020438485336064,
    ];

    let wasm = std::fs::read("wasm/rlp.wasm").unwrap();

    let loader = ZkWasmLoader::<Bn256, ExecutionArg, DefaultHostEnvBuilder>::new(20, wasm, vec![])?;
    let result = loader
        .run(
            ExecutionArg {
                public_inputs,
                private_inputs,
                context_inputs: vec![],
                context_outputs: Arc::new(Mutex::new(vec![])),
            },
            (),
            false,
            true,
        )
        .unwrap();

    let (circuit, instances) = loader.circuit_with_witness(result)?;
    Ok((loader, circuit, instances))
}

mod tests {
    use super::*;
    use rusty_fork::rusty_fork_test;

    rusty_fork_test! {
        #[test]
        fn test_rlp_mock() {
            let (loader, circuit, instances) = build_circuit().unwrap();

            loader.mock_test(&circuit, &instances).unwrap()
        }
    }

    rusty_fork_test! {
        #[test]
        fn test_rlp_bench() {
            let (loader, circuit, instances) = build_circuit().unwrap();

            loader.bench_test(circuit, instances)
        }
    }
}
