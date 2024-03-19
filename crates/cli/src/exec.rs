use anyhow::Result;
use circuits_batcher::args::HashType::Poseidon;
use circuits_batcher::proof::ParamsCache;
use circuits_batcher::proof::ProofGenerationInfo;
use circuits_batcher::proof::ProofInfo;
use circuits_batcher::proof::ProofPieceInfo;
use circuits_batcher::proof::ProvingKeyCache;
use delphinus_zkwasm::loader::ZkWasmLoader;
use delphinus_zkwasm::runtime::host::HostEnvBuilder;
use halo2_proofs::pairing::bn256::Bn256;
use halo2_proofs::plonk::verify_proof_with_shplonk;
use halo2_proofs::plonk::SingleVerifier;
use halo2_proofs::poly::commitment::ParamsVerifier;
use halo2aggregator_s::circuits::utils::load_or_build_unsafe_params;
use halo2aggregator_s::transcript::poseidon::PoseidonRead;
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

    let prover: ProofPieceInfo = ProofPieceInfo::new(prefix.to_string(), 0, instances.len() as u32);

    let mut param_cache = ParamsCache::<Bn256>::new(5, param_dir.clone());
    let mut pkey_cache = ProvingKeyCache::<Bn256>::new(5, param_dir.clone());

    let mut proof_load_info = ProofGenerationInfo::new(prefix, zkwasm_k as usize, Poseidon);

    let transcript = prover.exec_create_proof(
        &circuit,
        &vec![instances.clone()],
        zkwasm_k as usize,
        &mut pkey_cache,
        &mut param_cache,
        circuits_batcher::args::HashType::Poseidon,
        circuits_batcher::args::OpenSchema::Shplonk,
    );

    prover.save_proof_data(&vec![instances], &transcript, output_dir);

    //prover.mock_proof(k as u32);
    proof_load_info.append_single_proof(prover);
    proof_load_info.save(output_dir);

    info!("Proof has been created.");

    Ok(())
}

pub fn exec_verify_proof(
    prefix: &'static str,
    output_dir: &PathBuf,
    param_dir: &PathBuf,
) -> Result<()> {
    let load_info = output_dir.join(format!("{}.loadinfo.json", prefix));
    let proofloadinfo = ProofGenerationInfo::load(&load_info);
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
    for (_, proof) in proofs.into_iter().enumerate() {
        let strategy = SingleVerifier::new(&params_verifier);
        verify_proof_with_shplonk::<Bn256, _, _, _>(
            &params_verifier,
            &proof.vkey,
            strategy,
            &[&proof.instances.iter().map(|x| &x[..]).collect::<Vec<_>>()[..]],
            &mut PoseidonRead::init(&proof.transcripts[..]),
        )
        .unwrap();
    }
    info!("Verifing proof passed");

    Ok(())
}
