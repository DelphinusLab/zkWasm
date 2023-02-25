use crate::{
    foreign::wasm_input_helper::runtime::register_wasm_input_foreign,
    runtime::host::host_env::HostEnv,
};

use anyhow::Result;
use halo2_proofs::pairing::bn256::Fr as Fp;
use specs::Tables;
use std::fs::{self};

use super::test_circuit_with_env;

fn build_test() -> Result<(Tables, Vec<u64>)> {
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

    let wasm = fs::read("wasm/rlp.wasm").unwrap();

    let mut env = HostEnv::new();
    register_wasm_input_foreign(&mut env, public_inputs.clone(), private_inputs);
    env.finalize();

    let execution_result = test_circuit_with_env(
        env,
        wasm,
        "zkmain",
        public_inputs.iter().map(|v| Fp::from(*v)).collect(),
    )?;

    Ok((execution_result.tables, public_inputs))
}

mod tests {
    use super::*;
    use crate::{
        circuits::{config::set_zkwasm_k, ZkWasmCircuitBuilder},
        test::run_test_circuit,
    };
    use halo2_proofs::pairing::bn256::Fr as Fp;
    use rusty_fork::rusty_fork_test;

    rusty_fork_test! {
        #[test]
        fn test_rlp_mock() {
            set_zkwasm_k(20);

            let (tables, public_inputs) = build_test().unwrap();

            run_test_circuit(
                tables,
                public_inputs.into_iter().map(|v| Fp::from(v)).collect(),
            )
            .unwrap();
        }
    }

    rusty_fork_test! {
        #[test]
        fn test_rlp_bench() {
            set_zkwasm_k(20);

            let (tables, public_inputs) = build_test().unwrap();

            let builder = ZkWasmCircuitBuilder {
                tables,
            };

            builder.bench(public_inputs.into_iter().map(|v| Fp::from(v)).collect())
        }
    }
}
