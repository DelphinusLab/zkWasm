use anyhow::Result;
use circuits_batcher::proof::CircuitInfo;
use circuits_batcher::proof::ParamsCache;
use circuits_batcher::proof::ProofInfo;
use circuits_batcher::proof::ProofLoadInfo;
use circuits_batcher::proof::ProvingKeyCache;
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

pub fn exec_setup<Builder: HostEnvBuilder>(
    zkwasm_k: u32,
    aggregate_k: u32,
    prefix: &str,
    wasm_binary: Vec<u8>,
    phantom_functions: Vec<String>,
    envconfig: Builder::HostConfig,
    _output_dir: &PathBuf,
    param_dir: &PathBuf,
) -> Result<()> {
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
            let loader = ZkWasmLoader::<Bn256, Builder::Arg, Builder>::new(
                zkwasm_k,
                wasm_binary,
                phantom_functions,
            )?;

            let vkey = loader.create_vkey(&params, envconfig)?;

            let mut fd = std::fs::File::create(&vk_path)?;
            vkey.write(&mut fd)?;
        }
    }

    Ok(())
}

pub fn exec_image_checksum<Builder>(
    zkwasm_k: u32,
    wasm_binary: Vec<u8>,
    hostenv: Builder::HostConfig,
    phantom_functions: Vec<String>,
    output_dir: &PathBuf,
) -> Result<()>
where
    Builder: HostEnvBuilder,
{
    let loader = ZkWasmLoader::<Bn256, Builder::Arg, Builder>::new(
        zkwasm_k,
        wasm_binary,
        phantom_functions,
    )?;

    let params = load_or_build_unsafe_params::<Bn256>(
        zkwasm_k,
        Some(&output_dir.join(format!("K{}.params", zkwasm_k))),
    );

    let checksum = loader.checksum(&params, hostenv)?;
    assert_eq!(checksum.len(), 1);
    let checksum = checksum[0];

    println!("image checksum: {:?}", checksum);

    let mut fd =
        std::fs::File::create(&output_dir.join(format!("checksum.data",)).as_path()).unwrap();

    write!(fd, "{:?}", checksum)?;

    Ok(())
}

pub fn exec_dry_run<Builder: HostEnvBuilder>(
    zkwasm_k: u32,
    wasm_binary: Vec<u8>,
    phantom_functions: Vec<String>,
    arg: Builder::Arg,
    config: Builder::HostConfig,
) -> Result<()> {
    let loader = ZkWasmLoader::<Bn256, Builder::Arg, Builder>::new(
        zkwasm_k,
        wasm_binary,
        phantom_functions,
    )?;
    let result = loader.run(arg, config, true, false)?;
    println!("total guest instructions used {:?}", result.guest_statics);
    println!("total host api used {:?}", result.host_statics);
    Ok(())
}

pub fn exec_create_proof<Builder: HostEnvBuilder>(
    prefix: &'static str,
    zkwasm_k: u32,
    wasm_binary: Vec<u8>,
    phantom_functions: Vec<String>,
    output_dir: &PathBuf,
    param_dir: &PathBuf,
    arg: Builder::Arg,
    config: Builder::HostConfig,
) -> Result<()> {
    let loader = ZkWasmLoader::<Bn256, Builder::Arg, Builder>::new(
        zkwasm_k,
        wasm_binary,
        phantom_functions,
    )?;

    let execution_result = loader.run(arg, config, false, true)?;

    println!(
        "total guest instructions used {:?}",
        execution_result.guest_statics
    );
    println!("total host api used {:?}", execution_result.host_statics);
    println!("application outout {:?}", execution_result.outputs);

    let (circuit, instances) = loader.circuit_with_witness(execution_result)?;

    if false {
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

    // save the proof load info for the zkwasm circuit
    circuit.proofloadinfo.save(output_dir);

    // Cli saves zkwasm.0.instance.data as the
    // first instance file for .loadinfo
    // Thus we provide arg index = 0 to generate a
    // proof with the first instance file
    let mut param_cache = ParamsCache::new(5);
    let mut pkey_cache = ProvingKeyCache::new(5);

    circuit.exec_create_proof(output_dir, param_dir, &mut pkey_cache, 0, &mut param_cache);

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
