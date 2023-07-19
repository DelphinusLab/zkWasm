use clap::App;
use clap::AppSettings;
use log::info;
use log::warn;
use std::fs;
use std::path::PathBuf;

use crate::circuits::config::init_zkwasm_runtime;
use crate::circuits::config::MIN_K;
use crate::cli::exec::exec_dry_run;

use super::command::CommandBuilder;
use super::exec::compile_image;
use super::exec::exec_aggregate_create_proof;
use super::exec::exec_create_proof;
use super::exec::exec_dry_run_service;
#[cfg(feature = "checksum")]
use super::exec::exec_image_checksum;
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

        #[cfg(feature = "checksum")]
        let app = Self::append_image_checksum_subcommand(app);

        app
    }

    fn exec(command: App) {
        env_logger::init();

        let top_matches = command.get_matches();

        let zkwasm_k = Self::parse_zkwasm_k_arg(&top_matches).unwrap_or(MIN_K);

        let wasm_file_path = Self::parse_zkwasm_file_arg(&top_matches);
        let wasm_binary = fs::read(&wasm_file_path).unwrap();

        let function_name = Self::parse_function_name(&top_matches);

        {
            let module = wasmi::Module::from_buffer(&wasm_binary).expect("failed to load wasm");

            let (_, table) = compile_image(&module, &function_name);
            init_zkwasm_runtime(zkwasm_k, &table.tables);
        }

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
            #[cfg(feature = "checksum")]
            Some(("checksum", _)) => {
                exec_image_checksum(&wasm_binary, &function_name, &output_dir);
            }
            Some(("dry-run", sub_matches)) => {
                let public_inputs: Vec<u64> = Self::parse_single_public_arg(&sub_matches);
                let private_inputs: Vec<u64> = Self::parse_single_private_arg(&sub_matches);
                let service_mode = Self::parse_dry_run_service_arg(&sub_matches);

                if let Some(listen) = service_mode {
                    if !public_inputs.is_empty() || !private_inputs.is_empty() {
                        warn!("All private/public inputs are ignored when dry-run is running in service mode.");
                    }

                    exec_dry_run_service(wasm_binary, function_name, &listen).unwrap()
                } else {
                    assert!(public_inputs.len() <= Self::MAX_PUBLIC_INPUT_SIZE);

                    exec_dry_run(
                        &wasm_binary,
                        &function_name,
                        &public_inputs,
                        &private_inputs,
                    )
                    .unwrap();
                }
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
                let instance_path: PathBuf = Self::parse_single_instance_arg(&sub_matches);

                exec_verify_proof(
                    Self::NAME,
                    Self::MAX_PUBLIC_INPUT_SIZE,
                    zkwasm_k,
                    &wasm_binary,
                    &function_name,
                    &output_dir,
                    &proof_path,
                    &instance_path,
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
