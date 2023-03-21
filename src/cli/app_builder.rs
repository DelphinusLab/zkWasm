use clap::App;
use clap::AppSettings;
use log::info;
use std::fs;
use std::path::PathBuf;

use crate::circuits::config::set_zkwasm_k;
use crate::circuits::config::MIN_K;
use crate::cli::exec::exec_dry_run;

use super::command::CommandBuilder;
use super::exec::build_circuit_without_witness;
use super::exec::exec_aggregate_create_proof;
use super::exec::exec_create_proof;
use super::exec::exec_setup;
use super::exec::exec_solidity_aggregate_proof;
use super::exec::exec_verify_aggregate_proof;
use super::exec::exec_verify_proof;

fn load_or_generate_output_path(wasm_md5: &String, path: Option<&PathBuf>) -> PathBuf {
    if let Some(path) = path {
        path.clone()
    } else {
        info!("Output path is not provided, set to {}", wasm_md5);

        PathBuf::from(wasm_md5)
    }
}

pub trait AppBuilder: CommandBuilder {
    const NAME: &'static str;
    const VERSION: &'static str;
    const AGGREGATE_K: u32;
    const N_PROOFS: usize;
    const MAX_PUBLIC_INPUT_SIZE: usize;

    fn app_builder<'a>() -> App<'a> {
        let app = App::new(Self::NAME)
            .version(Self::VERSION)
            .setting(AppSettings::SubcommandRequired)
            .arg(Self::zkwasm_k_arg())
            .arg(Self::output_path_arg())
            .arg(Self::function_name_arg())
            .arg(Self::zkwasm_file_arg());

        let app = Self::append_setup_subcommand(app);
        let app = Self::append_dry_run_subcommand(app);
        let app = Self::append_create_single_proof_subcommand(app);
        let app = Self::append_verify_single_proof_subcommand(app);
        let app = Self::append_create_aggregate_proof_subcommand(app);
        let app = Self::append_verify_aggregate_verify_subcommand(app);
        let app = Self::append_generate_solidity_verifier(app);

        app
    }

    fn exec(command: App) {
        env_logger::init();

        let top_matches = command.get_matches();

        let zkwasm_k = Self::parse_zkwasm_k_arg(&top_matches).unwrap_or(MIN_K);
        set_zkwasm_k(zkwasm_k);

        let wasm_file_path = Self::parse_zkwasm_file_arg(&top_matches);
        let wasm_binary = fs::read(&wasm_file_path).unwrap();

        let function_name = Self::parse_function_name(&top_matches);

        /*
         * FIXME: trigger CIRCUIT_CONFIGURE initialization.
         */
        build_circuit_without_witness(&wasm_binary, &function_name);

        let md5 = format!("{:X}", md5::compute(&wasm_binary));

        let output_dir =
            load_or_generate_output_path(&md5, top_matches.get_one::<PathBuf>("output"));
        fs::create_dir_all(&output_dir).unwrap();

        match top_matches.subcommand() {
            Some(("setup", _)) => {
                exec_setup(
                    zkwasm_k,
                    Self::AGGREGATE_K,
                    Self::NAME,
                    &wasm_binary,
                    &function_name,
                    &output_dir,
                );
            }
            Some(("dry-run", sub_matches)) => {
                let public_inputs: Vec<u64> = Self::parse_single_public_arg(&sub_matches);
                let private_inputs: Vec<u64> = Self::parse_single_private_arg(&sub_matches);

                assert!(public_inputs.len() <= Self::MAX_PUBLIC_INPUT_SIZE);

                exec_dry_run(
                    &wasm_binary,
                    &function_name,
                    &public_inputs,
                    &private_inputs,
                )
                .unwrap();
            }
            Some(("single-prove", sub_matches)) => {
                let public_inputs: Vec<u64> = Self::parse_single_public_arg(&sub_matches);
                let private_inputs: Vec<u64> = Self::parse_single_private_arg(&sub_matches);

                assert!(public_inputs.len() <= Self::MAX_PUBLIC_INPUT_SIZE);

                exec_create_proof(
                    Self::NAME,
                    zkwasm_k,
                    &wasm_binary,
                    &function_name,
                    &output_dir,
                    &public_inputs,
                    &private_inputs,
                )
                .unwrap();
            }
            Some(("single-verify", sub_matches)) => {
                let proof_path: PathBuf = Self::parse_proof_path_arg(&sub_matches);
                let public_inputs: Vec<u64> = Self::parse_single_public_arg(&sub_matches);

                assert!(public_inputs.len() <= Self::MAX_PUBLIC_INPUT_SIZE);

                exec_verify_proof(
                    Self::NAME,
                    zkwasm_k,
                    &wasm_binary,
                    &function_name,
                    &output_dir,
                    &proof_path,
                    &public_inputs,
                );
            }
            Some(("aggregate-prove", sub_matches)) => {
                let public_inputs: Vec<Vec<u64>> = Self::parse_aggregate_public_args(&sub_matches);
                let private_inputs: Vec<Vec<u64>> =
                    Self::parse_aggregate_private_args(&sub_matches);

                for instances in &public_inputs {
                    assert!(instances.len() <= Self::MAX_PUBLIC_INPUT_SIZE);
                }

                assert_eq!(public_inputs.len(), Self::N_PROOFS);
                assert_eq!(private_inputs.len(), Self::N_PROOFS);

                exec_aggregate_create_proof(
                    zkwasm_k,
                    Self::AGGREGATE_K,
                    Self::NAME,
                    &wasm_binary,
                    &function_name,
                    &output_dir,
                    &public_inputs,
                    &private_inputs,
                );
            }

            Some(("aggregate-verify", sub_matches)) => {
                let proof_path: PathBuf = Self::parse_proof_path_arg(&sub_matches);
                let instances_path: PathBuf = Self::parse_aggregate_instance(&sub_matches);

                exec_verify_aggregate_proof(
                    Self::AGGREGATE_K as u32,
                    &output_dir,
                    &proof_path,
                    &instances_path,
                    Self::N_PROOFS,
                );
            }

            Some(("solidity-aggregate-verifier", sub_matches)) => {
                let proof_path: PathBuf = Self::parse_proof_path_arg(&sub_matches);
                let instances_path: PathBuf = Self::parse_aggregate_instance(&sub_matches);
                let aux_only: bool = Self::parse_auxonly(&sub_matches);
                let sol_path: PathBuf = Self::parse_sol_dir_arg(&sub_matches);

                exec_solidity_aggregate_proof(
                    zkwasm_k,
                    Self::AGGREGATE_K,
                    Self::MAX_PUBLIC_INPUT_SIZE,
                    &output_dir,
                    &proof_path,
                    &sol_path,
                    &instances_path,
                    Self::N_PROOFS,
                    aux_only,
                );
            }

            Some((_, _)) => todo!(),
            None => todo!(),
        }
    }
}
