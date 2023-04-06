use anyhow::Result;
use halo2_proofs::{
    dev::MockProver,
    pairing::bn256::{Bn256, Fr, G1Affine},
    plonk::{verify_proof, SingleVerifier},
    poly::commitment::ParamsVerifier,
};
use halo2aggregator_s::{
    circuit_verifier::circuit::AggregatorCircuit,
    circuits::utils::{
        load_instance, load_or_build_unsafe_params, load_or_build_vkey, load_or_create_proof,
        load_proof, load_vkey, run_circuit_unsafe_full_pass, TranscriptHash,
    },
    solidity_verifier::{codegen::solidity_aux_gen, solidity_render},
    transcript::{poseidon::PoseidonRead, sha256::ShaRead},
};
use log::info;
use specs::{ExecutionTable, Tables};
use std::path::PathBuf;
use wasmi::ImportsBuilder;

use crate::{
    circuits::{TestCircuit, ZkWasmCircuitBuilder},
    foreign::{
        log_helper::register_log_foreign, require_helper::register_require_foreign,
        sha256_helper::runtime::register_sha256_foreign,
        wasm_input_helper::runtime::register_wasm_input_foreign,
    },
    runtime::{host::host_env::HostEnv, wasmi_interpreter::Execution, WasmInterpreter},
};

const AGGREGATE_PREFIX: &'static str = "aggregate-circuit";

pub fn build_circuit_without_witness(
    wasm_binary: &Vec<u8>,
    function_name: &str,
) -> TestCircuit<Fr> {
    let module = wasmi::Module::from_buffer(wasm_binary).expect("failed to load wasm");

    let mut env = HostEnv::new();
    register_sha256_foreign(&mut env);
    register_wasm_input_foreign(&mut env, vec![], vec![]);
    register_require_foreign(&mut env);
    register_log_foreign(&mut env);
    env.finalize();
    let imports = ImportsBuilder::new().with_resolver("env", &env);

    let compiler = WasmInterpreter::new();
    let compiled_module = compiler
        .compile(
            &module,
            &imports,
            &env.function_description_table(),
            function_name,
        )
        .expect("file cannot be complied");

    let builder = ZkWasmCircuitBuilder {
        tables: Tables {
            compilation_tables: compiled_module.tables,
            execution_tables: ExecutionTable::default(),
        },
    };

    builder.build_circuit::<Fr>()
}

fn build_circuit_with_witness(
    wasm_binary: &Vec<u8>,
    function_name: &str,
    public_inputs: &Vec<u64>,
    private_inputs: &Vec<u64>,
) -> Result<TestCircuit<Fr>> {
    let module = wasmi::Module::from_buffer(wasm_binary).expect("failed to load wasm");

    let mut env = HostEnv::new();
    register_sha256_foreign(&mut env);
    register_wasm_input_foreign(&mut env, public_inputs.clone(), private_inputs.clone());
    register_require_foreign(&mut env);
    register_log_foreign(&mut env);
    env.finalize();
    let imports = ImportsBuilder::new().with_resolver("env", &env);

    let compiler = WasmInterpreter::new();
    let compiled_module = compiler
        .compile(
            &module,
            &imports,
            &env.function_description_table(),
            function_name,
        )
        .expect("file cannot be complied");

    let execution_result = compiled_module.run(&mut env)?;

    let builder = ZkWasmCircuitBuilder {
        tables: execution_result.tables,
    };

    Ok(builder.build_circuit())
}

pub fn exec_setup(
    zkwasm_k: u32,
    aggregate_k: u32,
    prefix: &'static str,
    wasm_binary: &Vec<u8>,
    entry: &str,
    output_dir: &PathBuf,
) {
    let circuit = build_circuit_without_witness(wasm_binary, entry);

    info!("Setup Params and VerifyingKey");

    // Setup ZkWasm Params
    let params = {
        let params_path = &output_dir.join(format!("K{}.params", zkwasm_k));

        if params_path.exists() {
            info!("Found Params with K = {} at {:?}", zkwasm_k, params_path);
        } else {
            info!("Create Params with K = {} to {:?}", zkwasm_k, params_path);
        }

        load_or_build_unsafe_params::<Bn256>(zkwasm_k, Some(params_path))
    };

    // Setup ZkWasm Vkey
    {
        let vk_path = &output_dir.join(format!("{}.{}.vkey.data", prefix, 0));

        if vk_path.exists() {
            info!("Found Verifying at {:?}", vk_path);
        } else {
            info!("Create Verifying to {:?}", vk_path);
        }

        load_or_build_vkey::<Bn256, _>(&params, &circuit, Some(vk_path));
    }

    // Setup Aggregate Circuit Params
    {
        let params_path = &output_dir.join(format!("K{}.params", aggregate_k));

        if params_path.exists() {
            info!("Found Params with K = {} at {:?}", aggregate_k, params_path);
        } else {
            info!(
                "Create Params with K = {} to {:?}",
                aggregate_k, params_path
            );
        }

        load_or_build_unsafe_params::<Bn256>(aggregate_k, Some(params_path))
    };
}

pub fn exec_dry_run(
    wasm_binary: &Vec<u8>,
    function_name: &str,
    public_inputs: &Vec<u64>,
    private_inputs: &Vec<u64>,
) -> Result<()> {
    let _ = build_circuit_with_witness(wasm_binary, function_name, public_inputs, private_inputs)?;

    info!("Execution passed.");

    Ok(())
}

pub fn exec_create_proof(
    prefix: &'static str,
    zkwasm_k: u32,
    wasm_binary: &Vec<u8>,
    function_name: &str,
    output_dir: &PathBuf,
    public_inputs: &Vec<u64>,
    private_inputs: &Vec<u64>,
) -> Result<()> {
    let circuit =
        build_circuit_with_witness(wasm_binary, function_name, public_inputs, private_inputs)?;
    let instances = vec![public_inputs
        .iter()
        .map(|v| Fr::from(*v))
        .collect::<Vec<_>>()
        .clone()];

    if true {
        info!("Mock test...");

        circuit.tables.write_json(Some(output_dir.clone()));

        let prover = MockProver::run(
            zkwasm_k,
            &circuit,
            vec![public_inputs.into_iter().map(|v| Fr::from(*v)).collect()],
        )?;

        assert_eq!(prover.verify(), Ok(()));

        info!("Mock test passed");
    }

    let params = load_or_build_unsafe_params::<Bn256>(
        zkwasm_k,
        Some(&output_dir.join(format!("K{}.params", zkwasm_k))),
    );

    let vkey = load_vkey::<Bn256, TestCircuit<_>>(
        &params,
        &output_dir.join(format!("{}.{}.vkey.data", prefix, 0)),
    );

    load_or_create_proof::<Bn256, _>(
        &params,
        vkey,
        circuit.clone(),
        &instances.iter().map(|x| &x[..]).collect::<Vec<_>>(),
        Some(&output_dir.join(format!("{}.{}.transcript.data", prefix, 0))),
        TranscriptHash::Poseidon,
        false,
    );

    info!("Proof has been created.");

    Ok(())
}

pub fn exec_verify_proof(
    prefix: &'static str,
    zkwasm_k: u32,
    output_dir: &PathBuf,
    proof_path: &PathBuf,
    public_inputs: &Vec<u64>,
) {
    let public_inputs_size = public_inputs.len();

    let instances = vec![public_inputs
        .iter()
        .map(|v| Fr::from(*v))
        .collect::<Vec<_>>()
        .clone()];

    let params = load_or_build_unsafe_params::<Bn256>(
        zkwasm_k,
        Some(&output_dir.join(format!("K{}.params", zkwasm_k))),
    );

    let vkey = load_vkey::<Bn256, TestCircuit<_>>(
        &params,
        &output_dir.join(format!("{}.{}.vkey.data", prefix, 0)),
    );

    let proof = load_proof(proof_path);

    let params_verifier: ParamsVerifier<Bn256> = params.verifier(public_inputs_size).unwrap();
    let strategy = SingleVerifier::new(&params_verifier);

    verify_proof(
        &params_verifier,
        &vkey,
        strategy,
        &[&instances.iter().map(|x| &x[..]).collect::<Vec<_>>()[..]],
        &mut PoseidonRead::init(&proof[..]),
    )
    .unwrap();

    info!("Verifing proof passed");
}

pub fn exec_aggregate_create_proof(
    zkwasm_k: u32,
    aggregate_k: u32,
    prefix: &'static str,
    wasm_binary: &Vec<u8>,
    function_name: &str,
    output_dir: &PathBuf,
    public_inputs: &Vec<Vec<u64>>,
    private_inputs: &Vec<Vec<u64>>,
) {
    assert_eq!(public_inputs.len(), private_inputs.len());

    let (circuits, instances) = public_inputs.iter().zip(private_inputs.iter()).fold(
        (vec![], vec![]),
        |(mut circuits, mut instances), (public, private)| {
            let circuit =
                build_circuit_with_witness(&wasm_binary, &function_name, &public, &private)
                    .unwrap();
            let instance = vec![public.iter().map(|v| Fr::from(*v)).collect()];

            circuits.push(circuit);
            instances.push(instance);

            (circuits, instances)
        },
    );

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
}

pub fn exec_verify_aggregate_proof(
    aggregate_k: u32,
    output_dir: &PathBuf,
    proof_path: &PathBuf,
    instances_path: &PathBuf,
    n_proofs: usize,
) {
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

    info!("Verifing Aggregate Proof Passed.")
}

const SOLIDITY_VERIFY_STEPS: usize = 2;

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
) {
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
            vec![
                vec!["AggregatorConfig.sol.tera".to_owned()],
                (0..SOLIDITY_VERIFY_STEPS)
                    .map(|i| format!("AggregatorVerifierStep{}.sol.tera", i + 1))
                    .into_iter()
                    .collect::<Vec<String>>(),
            ]
            .concat(),
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
}
