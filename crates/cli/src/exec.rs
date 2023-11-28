use anyhow::Result;
use circuits_batcher::proof::CircuitInfo;
use circuits_batcher::proof::ProofInfo;
use circuits_batcher::proof::ProofLoadInfo;
use delphinus_zkwasm::circuits::TestCircuit;
use delphinus_zkwasm::loader::ZkWasmLoader;
use delphinus_zkwasm::runtime::host::HostEnvBuilder;
use halo2_proofs::pairing::bn256::Bn256;
use halo2_proofs::pairing::bn256::Fr;
use halo2_proofs::poly::commitment::ParamsVerifier;
use halo2aggregator_s::circuits::utils::load_or_build_unsafe_params;
use halo2aggregator_s::circuits::utils::TranscriptHash;
use halo2aggregator_s::native_verifier;
use log::info;
use std::io::Write;
use std::path::PathBuf;

pub fn exec_setup<Arg, Builder>(
    zkwasm_k: u32,
    aggregate_k: u32,
    prefix: &str,
    wasm_binary: Vec<u8>,
    phantom_functions: Vec<String>,
    _output_dir: &PathBuf,
    param_dir: &PathBuf,
) -> Result<()>
where
    Builder: HostEnvBuilder<Arg = Arg>,
{
    info!("Setup Params and VerifyingKey");

    macro_rules! prepare_params {
        ($k: expr) => {{
            let params_path = &param_dir.join(format!("K{}.params", $k));

            if params_path.exists() {
                info!("Found Params with K = {} at {:?}", $k, params_path);
            } else {
                info!("Create Params with K = {} to {:?}", $k, params_path);
            }

            load_or_build_unsafe_params::<Bn256>($k, Some(params_path))
        }};
    }

    let params = prepare_params!(zkwasm_k);
    prepare_params!(aggregate_k);

    // Setup ZkWasm Vkey
    {
        let vk_path = &param_dir.join(format!("{}.vkey.data", prefix));

        if vk_path.exists() {
            info!("Found Verifying at {:?}", vk_path);
        } else {
            info!("Create Verifying to {:?}", vk_path);
            let loader =
                ZkWasmLoader::<Bn256, Arg, Builder>::new(zkwasm_k, wasm_binary, phantom_functions)?;

            let vkey = loader.create_vkey(&params)?;

            let mut fd = std::fs::File::create(&vk_path)?;
            vkey.write(&mut fd)?;
        }
    }

    Ok(())
}

pub fn exec_image_checksum<Arg, Builder>(
    zkwasm_k: u32,
    wasm_binary: Vec<u8>,
    phantom_functions: Vec<String>,
    output_dir: &PathBuf,
) -> Result<()>
where
    Builder: HostEnvBuilder<Arg = Arg>,
{
    let loader =
        ZkWasmLoader::<Bn256, Arg, Builder>::new(zkwasm_k, wasm_binary, phantom_functions)?;

    let params = load_or_build_unsafe_params::<Bn256>(
        zkwasm_k,
        Some(&output_dir.join(format!("K{}.params", zkwasm_k))),
    );

    let checksum = loader.checksum(&params)?;
    assert_eq!(checksum.len(), 1);
    let checksum = checksum[0];

    println!("image checksum: {:?}", checksum);

    let mut fd =
        std::fs::File::create(&output_dir.join(format!("checksum.data",)).as_path()).unwrap();

    write!(fd, "{:?}", checksum)?;

    Ok(())
}

pub fn exec_dry_run<Arg, Builder: HostEnvBuilder<Arg = Arg>>(
    zkwasm_k: u32,
    wasm_binary: Vec<u8>,
    phantom_functions: Vec<String>,
    arg: Arg,
) -> Result<()> {
    let loader =
        ZkWasmLoader::<Bn256, Arg, Builder>::new(zkwasm_k, wasm_binary, phantom_functions)?;
    loader.run(arg, true, false)?;
    Ok(())
}

pub fn exec_create_proof<Arg, Builder: HostEnvBuilder<Arg = Arg>>(
    prefix: &'static str,
    zkwasm_k: u32,
    wasm_binary: Vec<u8>,
    phantom_functions: Vec<String>,
    output_dir: &PathBuf,
    param_dir: &PathBuf,
    arg: Arg,
) -> Result<()> {
    let loader =
        ZkWasmLoader::<Bn256, Arg, Builder>::new(zkwasm_k, wasm_binary, phantom_functions)?;

    let (circuit, instances, _) = loader.circuit_with_witness(arg)?;

    if true {
        info!("Mock test...");
        loader.mock_test(&circuit, &instances)?;
        info!("Mock test passed");
    }

    let circuit: CircuitInfo<Bn256, TestCircuit<Fr>> = CircuitInfo::new(
        circuit,
        prefix.to_string(),
        vec![instances],
        zkwasm_k as usize,
        circuits_batcher::args::HashType::Poseidon,
    );
    circuit.proofloadinfo.save(output_dir);
    circuit.exec_create_proof(output_dir, param_dir, 0);

    info!("Proof has been created.");

    Ok(())
}

pub fn exec_verify_proof(
    prefix: &'static str,
    output_dir: &PathBuf,
    param_dir: &PathBuf,
) -> Result<()> {
    let load_info = output_dir.join(format!("{}.loadinfo.json", prefix));
    let proofloadinfo = ProofLoadInfo::load(&load_info);
    let proofs: Vec<ProofInfo<Bn256>> =
        ProofInfo::load_proof(&output_dir, &param_dir, &proofloadinfo);
    let params = load_or_build_unsafe_params::<Bn256>(
        proofloadinfo.k as u32,
        Some(&param_dir.join(format!("K{}.params", proofloadinfo.k))),
    );
    let mut public_inputs_size = 0;
    for proof in proofs.iter() {
        public_inputs_size = usize::max(
            public_inputs_size,
            proof
                .instances
                .iter()
                .fold(0, |acc, x| usize::max(acc, x.len())),
        );
    }

    let params_verifier: ParamsVerifier<Bn256> = params.verifier(public_inputs_size).unwrap();
    for (_, proof) in proofs.iter().enumerate() {
        native_verifier::verify_single_proof::<Bn256>(
            &params_verifier,
            &proof.vkey,
            &proof.instances,
            proof.transcripts.clone(),
            TranscriptHash::Poseidon,
        );
    }
    info!("Verifing proof passed");

    Ok(())
}
