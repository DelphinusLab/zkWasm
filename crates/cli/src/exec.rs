use crate::app_builder::write_context_output;
use crate::args::parse_args;
use anyhow::Result;
use delphinus_zkwasm::circuits::TestCircuit;
use delphinus_zkwasm::halo2_proofs;
use delphinus_zkwasm::halo2aggregator_s;
use delphinus_zkwasm::loader::ExecutionArg;
use delphinus_zkwasm::loader::ZkWasmLoader;
use halo2_proofs::arithmetic::BaseExt;
use halo2_proofs::pairing::bn256::Bn256;
use halo2_proofs::pairing::bn256::Fr;
use halo2_proofs::pairing::bn256::G1Affine;
use halo2_proofs::plonk::verify_proof;
use halo2_proofs::plonk::SingleVerifier;
use halo2_proofs::poly::commitment::ParamsVerifier;
use halo2aggregator_s::circuit_verifier::circuit::AggregatorCircuit;
use halo2aggregator_s::circuits::utils::load_instance;
use halo2aggregator_s::circuits::utils::load_or_build_unsafe_params;
use halo2aggregator_s::circuits::utils::load_proof;
use halo2aggregator_s::circuits::utils::load_vkey;
use halo2aggregator_s::circuits::utils::run_circuit_unsafe_full_pass;
use halo2aggregator_s::circuits::utils::store_instance;
use halo2aggregator_s::circuits::utils::TranscriptHash;
use halo2aggregator_s::solidity_verifier::codegen::solidity_aux_gen;
use halo2aggregator_s::solidity_verifier::solidity_render;
use halo2aggregator_s::transcript::sha256::ShaRead;
use log::debug;
use log::error;
use log::info;
use notify::event::AccessMode;
use notify::RecursiveMode;
use notify::Watcher;
use serde::Deserialize;
use serde::Serialize;
use std::cell::RefCell;
use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;
use std::rc::Rc;
use wasmi::RuntimeValue;

const AGGREGATE_PREFIX: &'static str = "aggregate-circuit";

pub fn exec_setup(
    zkwasm_k: u32,
    aggregate_k: u32,
    prefix: &str,
    wasm_binary: Vec<u8>,
    phantom_functions: Vec<String>,
    output_dir: &PathBuf,
) -> Result<()> {
    info!("Setup Params and VerifyingKey");

    macro_rules! prepare_params {
        ($k: expr) => {{
            let params_path = &output_dir.join(format!("K{}.params", $k));

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
        let vk_path = &output_dir.join(format!("{}.{}.vkey.data", prefix, 0));

        if vk_path.exists() {
            info!("Found Verifying at {:?}", vk_path);
        } else {
            info!("Create Verifying to {:?}", vk_path);
            let loader = ZkWasmLoader::<Bn256>::new(zkwasm_k, wasm_binary, phantom_functions)?;

            let vkey = loader.create_vkey(&params)?;

            let mut fd = std::fs::File::create(&vk_path)?;
            vkey.write(&mut fd)?;
        }
    }

    Ok(())
}

#[cfg(feature = "checksum")]
pub fn exec_image_checksum(
    zkwasm_k: u32,
    wasm_binary: Vec<u8>,
    phantom_functions: Vec<String>,
    output_dir: &PathBuf,
) -> Result<()> {
    let loader = ZkWasmLoader::<Bn256>::new(zkwasm_k, wasm_binary, phantom_functions)?;

    let hash: Fr = loader.checksum()?;

    let mut fd =
        std::fs::File::create(&output_dir.join(format!("checksum.data",)).as_path()).unwrap();

    let hash = hash.to_string();
    write!(fd, "{}", hash).unwrap();
    println!("{}", hash);

    Ok(())
}

pub fn exec_dry_run_service(
    zkwasm_k: u32,
    wasm_binary: Vec<u8>,
    phantom_functions: Vec<String>,
    listen: &PathBuf,
) -> Result<()> {
    use notify::event::AccessKind;
    use notify::event::EventKind;
    use notify::event::ModifyKind;
    use notify::event::RenameMode;
    use notify::Event;

    #[derive(Serialize, Deserialize, Debug)]
    struct Sequence {
        private_inputs: Vec<String>,
        public_inputs: Vec<String>,
        context_input: Vec<String>,
        context_output: Option<PathBuf>,
    }

    info!("Dry-run service is running.");
    info!("{:?} is watched", listen);

    let mut watcher =
        notify::recommended_watcher(move |handler: Result<Event, _>| match handler {
            Ok(event) => {
                debug!("Event {:?}", event);

                match event.kind {
                    EventKind::Access(AccessKind::Close(AccessMode::Write))
                    | EventKind::Modify(ModifyKind::Name(RenameMode::To)) => {
                        assert_eq!(event.paths.len(), 1);
                        let path = event.paths.first().unwrap();

                        if let Some(ext) = path.extension() {
                            if ext.eq("done") {
                                return;
                            };
                        }

                        info!("Receive a request from file {:?}", path);

                        let json = fs::read_to_string(path).unwrap();
                        if let Ok(sequence) = serde_json::from_str::<Sequence>(&json) {
                            debug!("{:?}", sequence);

                            let private_inputs = parse_args(
                                sequence.private_inputs.iter().map(|s| s.as_str()).collect(),
                            );
                            let public_inputs = parse_args(
                                sequence.public_inputs.iter().map(|s| s.as_str()).collect(),
                            );
                            let context_inputs = parse_args(
                                sequence.context_input.iter().map(|s| s.as_str()).collect(),
                            );
                            let context_outputs = Rc::new(RefCell::new(vec![]));
                            let external_outputs = Rc::new(RefCell::new(HashMap::new()));

                            let loader = ZkWasmLoader::<Bn256>::new(
                                zkwasm_k,
                                wasm_binary.clone(),
                                phantom_functions.clone(),
                            )
                            .unwrap();

                            let r = loader
                                .dry_run(ExecutionArg {
                                    public_inputs,
                                    private_inputs,
                                    context_inputs,
                                    context_outputs: context_outputs.clone(),
                                    external_outputs: external_outputs.clone(),
                                })
                                .unwrap();
                            println!("return value: {:?}", r);

                            log::info!("external outputs {:?}", external_outputs);

                            write_context_output(
                                &context_outputs.borrow().to_vec(),
                                sequence.context_output,
                            )
                            .unwrap();

                            fs::write(
                                Path::new(&format!("{}.done", path.to_str().unwrap())),
                                if let Some(r) = r {
                                    match r {
                                        RuntimeValue::I32(v) => v.to_string(),
                                        RuntimeValue::I64(v) => v.to_string(),
                                        _ => unreachable!(),
                                    }
                                } else {
                                    "".to_owned()
                                },
                            )
                            .unwrap();
                        } else {
                            error!("Failed to parse file {:?}, the request is ignored.", path);
                        }
                    }
                    _ => (),
                }
            }
            Err(e) => println!("watch error: {:?}", e),
        })?;

    loop {
        watcher.watch(listen.as_path(), RecursiveMode::NonRecursive)?;
    }
}

pub fn exec_dry_run(
    zkwasm_k: u32,
    wasm_binary: Vec<u8>,
    phantom_functions: Vec<String>,
    public_inputs: Vec<u64>,
    private_inputs: Vec<u64>,
    context_inputs: Vec<u64>,
    context_outputs: Rc<RefCell<Vec<u64>>>,
    external_outputs: Rc<RefCell<HashMap<u64, Vec<u64>>>>,
) -> Result<()> {
    let loader = ZkWasmLoader::<Bn256>::new(zkwasm_k, wasm_binary, phantom_functions)?;

    loader.dry_run(ExecutionArg {
        public_inputs,
        private_inputs,
        context_inputs,
        context_outputs,
        external_outputs,
    })?;

    Ok(())
}

pub fn exec_create_proof(
    prefix: &'static str,
    zkwasm_k: u32,
    wasm_binary: Vec<u8>,
    phantom_functions: Vec<String>,
    output_dir: &PathBuf,
    public_inputs: Vec<u64>,
    private_inputs: Vec<u64>,
    context_inputs: Vec<u64>,
    context_outputs: Rc<RefCell<Vec<u64>>>,
    external_outputs: Rc<RefCell<HashMap<u64, Vec<u64>>>>,
) -> Result<()> {
    let loader = ZkWasmLoader::<Bn256>::new(zkwasm_k, wasm_binary, phantom_functions)?;

    let params = load_or_build_unsafe_params::<Bn256>(
        zkwasm_k,
        Some(&output_dir.join(format!("K{}.params", zkwasm_k))),
    );

    let vkey = load_vkey::<Bn256, TestCircuit<_>>(
        &params,
        &output_dir.join(format!("{}.{}.vkey.data", prefix, 0)),
    );

    let (circuit, instances) = loader.circuit_with_witness(ExecutionArg {
        public_inputs,
        private_inputs,
        context_inputs,
        context_outputs,
        external_outputs,
    })?;

    {
        store_instance(
            &vec![instances.clone()],
            &output_dir.join(format!("{}.{}.instance.data", prefix, 0)),
        );
    }

    if true {
        info!("Mock test...");
        loader.mock_test(&circuit, &instances)?;
        info!("Mock test passed");
    }

    let proof = loader.create_proof(&params, vkey, circuit, instances)?;

    {
        let proof_path = output_dir.join(format!("{}.{}.transcript.data", prefix, 0));
        println!("write transcript to {:?}", proof_path);
        let mut fd = std::fs::File::create(&proof_path)?;
        fd.write_all(&proof)?;
    }

    info!("Proof has been created.");

    Ok(())
}

pub fn exec_verify_proof(
    prefix: &'static str,
    zkwasm_k: u32,
    wasm_binary: Vec<u8>,
    phantom_functions: Vec<String>,
    output_dir: &PathBuf,
    proof_path: &PathBuf,
    instance_path: &PathBuf,
) -> Result<()> {
    let instances = {
        let mut instance = vec![];
        let mut fd = std::fs::File::open(&instance_path).unwrap();
        while let Ok(f) = Fr::read(&mut fd) {
            instance.push(f);
        }

        instance
    };
    let params = load_or_build_unsafe_params::<Bn256>(
        zkwasm_k,
        Some(&output_dir.join(format!("K{}.params", zkwasm_k))),
    );

    let loader = ZkWasmLoader::<Bn256>::new(zkwasm_k, wasm_binary, phantom_functions)?;

    let vkey = load_vkey::<Bn256, TestCircuit<_>>(
        &params,
        &output_dir.join(format!("{}.{}.vkey.data", prefix, 0)),
    );

    let proof = load_proof(proof_path);

    loader.verify_proof(&params, &vkey, &instances, &proof)?;

    info!("Verifing proof passed");

    Ok(())
}

pub fn exec_aggregate_create_proof(
    zkwasm_k: u32,
    aggregate_k: u32,
    prefix: &'static str,
    wasm_binary: Vec<u8>,
    phantom_functions: Vec<String>,
    output_dir: &PathBuf,
    public_inputs: Vec<Vec<u64>>,
    private_inputs: Vec<Vec<u64>>,
    context_inputs: Vec<Vec<u64>>,
    context_outputs: Vec<Rc<RefCell<Vec<u64>>>>,
) -> Result<()> {
    assert_eq!(public_inputs.len(), private_inputs.len());

    let loader = ZkWasmLoader::<Bn256>::new(zkwasm_k, wasm_binary, phantom_functions)?;

    let (circuits, instances) = public_inputs
        .into_iter()
        .zip(private_inputs.into_iter())
        .zip(context_inputs.into_iter())
        .zip(context_outputs.into_iter())
        .fold(
            Ok::<_, anyhow::Error>((vec![], vec![])),
            |acc, (((public_inputs, private_inputs), context_inputs), context_outputs)| {
                acc.and_then(|(mut circuits, mut instances)| {
                    let (circuit, instance) = loader.circuit_with_witness(ExecutionArg {
                        public_inputs,
                        private_inputs,
                        context_inputs,
                        context_outputs,
                        external_outputs: Rc::new(RefCell::new(HashMap::new())),
                    })?;

                    circuits.push(circuit);
                    instances.push(vec![instance]);

                    Ok((circuits, instances))
                })
            },
        )?;

    let (aggregate_circuit, aggregate_instances) = run_circuit_unsafe_full_pass::<Bn256, _>(
        &output_dir.as_path(),
        prefix,
        zkwasm_k,
        circuits,
        instances,
        TranscriptHash::Poseidon,
        vec![],
        false,
    )
    .unwrap();

    run_circuit_unsafe_full_pass::<Bn256, _>(
        &output_dir.as_path(),
        AGGREGATE_PREFIX,
        aggregate_k,
        vec![aggregate_circuit],
        vec![vec![aggregate_instances]],
        TranscriptHash::Sha,
        vec![],
        true,
    );

    Ok(())
}

pub fn exec_verify_aggregate_proof(
    aggregate_k: u32,
    output_dir: &PathBuf,
    proof_path: &PathBuf,
    instances_path: &PathBuf,
    n_proofs: usize,
) -> Result<()> {
    let params = load_or_build_unsafe_params::<Bn256>(
        aggregate_k,
        Some(&output_dir.join(format!("K{}.params", aggregate_k))),
    );

    let proof = load_proof(&proof_path.as_path());
    let vkey = load_vkey::<Bn256, AggregatorCircuit<G1Affine>>(
        &params,
        &output_dir.join(format!("{}.{}.vkey.data", AGGREGATE_PREFIX, 0)),
    );

    let public_inputs_size: u32 = 6 + 3 * n_proofs as u32;

    let instances = load_instance::<Bn256>(&[public_inputs_size], &instances_path);

    let params_verifier: ParamsVerifier<Bn256> =
        params.verifier(public_inputs_size as usize).unwrap();
    let strategy = SingleVerifier::new(&params_verifier);

    verify_proof(
        &params_verifier,
        &vkey,
        strategy,
        &[&instances.iter().map(|x| &x[..]).collect::<Vec<_>>()[..]],
        &mut ShaRead::<_, _, _, sha2::Sha256>::init(&proof[..]),
    )
    .unwrap();

    info!("Verifing Aggregate Proof Passed.");

    Ok(())
}

pub fn exec_solidity_aggregate_proof(
    zkwasm_k: u32,
    aggregate_k: u32,
    max_public_inputs_size: usize,
    output_dir: &PathBuf,
    proof_path: &PathBuf,
    sol_path: &PathBuf,
    instances_path: &PathBuf,
    n_proofs: usize,
    aux_only: bool,
) -> Result<()> {
    let zkwasm_params_verifier: ParamsVerifier<Bn256> = {
        let params = load_or_build_unsafe_params::<Bn256>(
            zkwasm_k,
            Some(&output_dir.join(format!("K{}.params", zkwasm_k))),
        );

        params.verifier(max_public_inputs_size).unwrap()
    };

    let (verifier_params_verifier, vkey, instances, proof) = {
        let public_inputs_size = 6 + 3 * n_proofs;

        let params = load_or_build_unsafe_params::<Bn256>(
            aggregate_k,
            Some(&output_dir.join(format!("K{}.params", aggregate_k))),
        );

        let params_verifier = params.verifier(public_inputs_size).unwrap();

        let vkey = load_vkey::<Bn256, AggregatorCircuit<G1Affine>>(
            &params,
            &output_dir.join(format!("{}.{}.vkey.data", AGGREGATE_PREFIX, 0)),
        );

        let instances = load_instance::<Bn256>(&[public_inputs_size as u32], &instances_path);
        let proof = load_proof(&proof_path.as_path());

        (params_verifier, vkey, instances, proof)
    };

    if !aux_only {
        let path_in = {
            let mut path = sol_path.clone();
            path.push("templates");
            path
        };
        let path_out = {
            let mut path = sol_path.clone();
            path.push("contracts");
            path
        };
        solidity_render(
            &(path_in.to_str().unwrap().to_owned() + "/*"),
            path_out.to_str().unwrap(),
            vec![(
                "AggregatorConfig.sol.tera".to_owned(),
                "AggregatorConfig.sol".to_owned(),
            )],
            "AggregatorVerifierStepStart.sol.tera",
            "AggregatorVerifierStepEnd.sol.tera",
            |i| format!("AggregatorVerifierStep{}.sol", i + 1),
            &zkwasm_params_verifier,
            &verifier_params_verifier,
            &vkey,
            &instances[0],
            proof.clone(),
        );
    }

    solidity_aux_gen(
        &verifier_params_verifier,
        &vkey,
        &instances[0],
        proof,
        &output_dir.join(format!("{}.{}.aux.data", AGGREGATE_PREFIX, 0)),
    );

    Ok(())
}
