use anyhow::Result;
use clap::App;
use clap::AppSettings;
use delphinus_zkwasm::circuits::config::MIN_K;
use delphinus_zkwasm::runtime::host::default_env::DefaultHostEnvBuilder;
use delphinus_zkwasm::runtime::host::default_env::ExecutionArg;
use log::info;
use log::warn;
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;

use crate::exec::exec_dry_run;

use super::command::CommandBuilder;
use super::exec::exec_create_proof;
use super::exec::exec_dry_run_service;
use super::exec::exec_image_checksum;
use super::exec::exec_setup;
use super::exec::exec_verify_proof;

fn load_or_generate_output_path(wasm_md5: &String, path: Option<&PathBuf>) -> PathBuf {
    if let Some(path) = path {
        path.clone()
    } else {
        info!("Output path is not provided, set to {}", wasm_md5);

        PathBuf::from(wasm_md5)
    }
}

pub fn write_context_output(
    context_output: &Vec<u64>,
    context_out_path: Option<PathBuf>,
) -> Result<()> {
    if let Some(path) = context_out_path {
        let mut fd = fs::File::create(path.as_path())?;
        fd.write_all("0x".as_bytes())?;

        for value in context_output {
            let bytes = value.to_le_bytes();
            let s = hex::encode(bytes);
            fd.write_all(&s.as_bytes())?;
        }

        fd.write_all(":bytes-packed".as_bytes())?;
    }

    Ok(())
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
            .arg(Self::param_path_arg())
            .arg(Self::function_name_arg())
            .arg(Self::phantom_functions_arg())
            .arg(Self::zkwasm_file_arg());

        let app = Self::append_setup_subcommand(app);
        let app = Self::append_dry_run_subcommand(app);
        let app = Self::append_create_single_proof_subcommand(app);
        let app = Self::append_verify_single_proof_subcommand(app);
        let app = Self::append_image_checksum_subcommand(app);

        app
    }

    fn exec(command: App) -> Result<()> {
        env_logger::init();

        let top_matches = command.get_matches();

        let zkwasm_k = Self::parse_zkwasm_k_arg(&top_matches).unwrap_or(MIN_K);

        let wasm_file_path = Self::parse_zkwasm_file_arg(&top_matches);
        let wasm_binary = fs::read(&wasm_file_path).unwrap();

        let function_name = Self::parse_function_name(&top_matches);
        assert_eq!(function_name, "zkmain");

        let md5 = format!("{:X}", md5::compute(&wasm_binary));
        let phantom_functions = Self::parse_phantom_functions(&top_matches);

        let param_dir =
            load_or_generate_output_path(&md5, top_matches.get_one::<PathBuf>("param"));

        let output_dir =
            load_or_generate_output_path(&md5, top_matches.get_one::<PathBuf>("output"));
        fs::create_dir_all(&output_dir)?;

        match top_matches.subcommand() {
            Some(("setup", _)) => exec_setup::<ExecutionArg, DefaultHostEnvBuilder>(
                zkwasm_k,
                Self::AGGREGATE_K,
                Self::NAME,
                wasm_binary,
                phantom_functions,
                &output_dir,
                &param_dir,
            ),
            Some(("checksum", _)) => {
                exec_image_checksum::<ExecutionArg, DefaultHostEnvBuilder>(zkwasm_k, wasm_binary, phantom_functions, &output_dir)
            }
            Some(("dry-run", sub_matches)) => {
                let public_inputs: Vec<u64> = Self::parse_single_public_arg(&sub_matches);
                let private_inputs: Vec<u64> = Self::parse_single_private_arg(&sub_matches);
                let context_in: Vec<u64> = Self::parse_context_in_arg(&sub_matches);
                let context_out_path: Option<PathBuf> =
                    Self::parse_context_out_path_arg(&sub_matches);
                let service_mode = Self::parse_dry_run_service_arg(&sub_matches);

                if let Some(listen) = service_mode {
                    if !public_inputs.is_empty() || !private_inputs.is_empty() {
                        warn!("All private/public inputs are ignored when dry-run is running in service mode.");
                    }

                    if !context_in.is_empty() || context_out_path.is_some() {
                        warn!("All context paths are ignored when dry-run is running in service mode.");
                    }

                    exec_dry_run_service(
                        zkwasm_k,
                        wasm_binary,
                        phantom_functions,
                        &listen
                    )
                } else {
                    assert!(public_inputs.len() <= Self::MAX_PUBLIC_INPUT_SIZE);

                    let context_output = Arc::new(Mutex::new(vec![]));

                    exec_dry_run::<ExecutionArg, DefaultHostEnvBuilder>(
                        zkwasm_k,
                        wasm_binary,
                        phantom_functions,
                        ExecutionArg {
                            public_inputs,
                            private_inputs,
                            context_inputs: context_in,
                            context_outputs: Arc::new(Mutex::new(vec![]))
                        }
                    )?;

                    write_context_output(&context_output.lock().unwrap(), context_out_path)?;

                    Ok(())
                }
            }
            Some(("single-prove", sub_matches)) => {
                let public_inputs: Vec<u64> = Self::parse_single_public_arg(&sub_matches);
                let private_inputs: Vec<u64> = Self::parse_single_private_arg(&sub_matches);
                let context_in: Vec<u64> = Self::parse_context_in_arg(&sub_matches);
                let context_out_path: Option<PathBuf> =
                    Self::parse_context_out_path_arg(&sub_matches);

                let context_out = Arc::new(Mutex::new(vec![]));

                assert!(public_inputs.len() <= Self::MAX_PUBLIC_INPUT_SIZE);

                exec_create_proof::<ExecutionArg, DefaultHostEnvBuilder>(
                    Self::NAME,
                    zkwasm_k,
                    wasm_binary,
                    phantom_functions,
                    &output_dir,
                    &param_dir,
                    ExecutionArg {
                        public_inputs,
                        private_inputs,
                        context_inputs: context_in,
                        context_outputs: context_out.clone(),
                    }
                )?;

                write_context_output(&context_out.lock().unwrap(), context_out_path)?;

                Ok(())
            }
            Some(("single-verify", _)) => {
                exec_verify_proof(
                    Self::NAME,
                    &output_dir,
                    &param_dir
                )
            }
            Some((_, _)) => todo!(),
            None => todo!(),
        }
    }
}
