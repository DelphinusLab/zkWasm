#![deny(warnings)]

use std::cell::RefCell;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::rc::Rc;

use anyhow::Result;
use clap::Parser;
use clap::Subcommand;
use delphinus_host::StandardHostEnvBuilder;
use delphinus_zkwasm::foreign::context::ContextOutput;
use delphinus_zkwasm::runtime::host::default_env::DefaultHostEnvBuilder;
use delphinus_zkwasm::runtime::host::default_env::ExecutionArg;

use crate::config::Config;
use args::HostMode;
use names::name_of_config;
use specs::args::parse_args;

mod args;
mod config;
mod names;

#[derive(Subcommand)]
enum Subcommands {
    /// Setup a new zkWasm circuit for provided Wasm image.
    Setup {
        /// Size of the circuit.
        #[clap(short, default_value = "18")]
        k: u32,

        /// Path to the Wasm image.
        #[clap(long = "wasm")]
        wasm_image: PathBuf,

        /// Specify execution host envionment for the runtime.
        #[clap(long = "host", default_value_t, value_enum)]
        host_mode: HostMode,

        #[clap(long = "phantom")]
        /// Specify phantom functions whose body will be ignored in the circuit.
        phantom_functions: Vec<String>,
    },
    /// Execute the Wasm image without generating a proof.
    DryRun {
        /// Path to the Wasm image.
        #[clap(long = "wasm")]
        wasm_image: PathBuf,

        /// Path to the directory to write the output.
        #[clap(short = 'o', long = "output")]
        output_dir: PathBuf,

        /// Public inputs with format 'value:type' where type=i64|bytes|bytes-packed|file.
        #[clap(long = "public")]
        public_inputs: Vec<String>,

        /// Private inputs with format 'value:type' where type=i64|bytes|bytes-packed|file.
        #[clap(long = "private")]
        private_inputs: Vec<String>,

        /// Context inputs with format 'value:type' where type=i64|bytes|bytes-packed|file.
        #[clap(long = "context-in")]
        context_inputs: Vec<String>,

        /// Filename to the file to write the context output.
        #[clap(long = "context-out")]
        context_output: Option<String>,
    },
    /// Execute the Wasm image and generate a proof.
    Prove {
        /// Path to the Wasm image.
        #[clap(long = "wasm")]
        wasm_image: PathBuf,

        /// Path to the directory to proof.
        #[clap(short = 'o', long = "output")]
        output_dir: PathBuf,

        /// Public inputs with format 'value:type' where type=i64|bytes|bytes-packed|file.
        #[clap(long = "public")]
        public_inputs: Vec<String>,

        /// Private inputs with format 'value:type' where type=i64|bytes|bytes-packed|file.
        #[clap(long = "private")]
        private_inputs: Vec<String>,

        /// Context inputs with format 'value:type' where type=i64|bytes|bytes-packed|file.
        #[clap(long = "context-in")]
        context_inputs: Vec<String>,

        /// Filename to the file to write the context output.
        #[clap(long = "context-out")]
        context_output: Option<String>,

        /// Enable mock test.
        #[clap(long = "mock", default_value = "false", takes_value = false)]
        mock_test: bool,
    },
    /// Verify the proof.
    Verify {
        /// Path to the directory to proof.
        #[clap(short = 'o', long = "output")]
        output_dir: PathBuf,
    },
}

#[derive(Parser)]
struct ZkWasmCli {
    /// Name of the configuration.
    name: String,

    /// Directory to setup params and configuration.
    #[clap(long = "params")]
    params_dir: PathBuf,

    #[clap(subcommand)]
    subcommand: Subcommands,
}

/// Simple program to greet a person
fn main() -> Result<()> {
    {
        env_logger::init();
    }

    let cli = ZkWasmCli::parse();

    match cli.subcommand {
        Subcommands::Setup {
            k,
            wasm_image,
            host_mode,
            phantom_functions,
        } => {
            let wasm_binary = fs::read(&wasm_image)?;

            match host_mode {
                HostMode::DEFAULT => {
                    Config::setup::<DefaultHostEnvBuilder>(
                        &cli.name,
                        k,
                        wasm_binary,
                        phantom_functions,
                        host_mode,
                        &cli.params_dir,
                    )?;
                }
                HostMode::STANDARD => {
                    Config::setup::<StandardHostEnvBuilder>(
                        &cli.name,
                        k,
                        wasm_binary,
                        phantom_functions,
                        host_mode,
                        &cli.params_dir,
                    )?;
                }
            }
        }

        Subcommands::Prove {
            wasm_image,
            output_dir,
            public_inputs,
            private_inputs,
            context_inputs,
            context_output,
            mock_test,
        } => {
            let config = Config::read(&mut fs::File::open(
                cli.params_dir.join(&name_of_config(&cli.name)),
            )?)?;

            fs::create_dir_all(&output_dir)?;

            let public_inputs = parse_args(&public_inputs);
            let private_inputs = parse_args(&private_inputs);
            let context_inputs = parse_args(&context_inputs);

            match config.host_mode {
                HostMode::DEFAULT => {
                    config.prove::<DefaultHostEnvBuilder>(
                        &wasm_image,
                        &cli.params_dir,
                        &output_dir,
                        ExecutionArg {
                            public_inputs,
                            private_inputs,
                            context_inputs,
                            context_outputs: ContextOutput::default(),
                        },
                        context_output,
                        mock_test,
                    )?;
                }
                HostMode::STANDARD => {
                    config.prove::<StandardHostEnvBuilder>(
                        &wasm_image,
                        &cli.params_dir,
                        &output_dir,
                        delphinus_host::ExecutionArg {
                            public_inputs,
                            private_inputs,
                            context_inputs,
                            context_outputs: ContextOutput::default(),
                            indexed_witness: Rc::new(RefCell::new(HashMap::new())),
                            tree_db: None,
                        },
                        context_output,
                        mock_test,
                    )?;
                }
            }
        }

        Subcommands::DryRun {
            wasm_image,
            output_dir,
            public_inputs,
            private_inputs,
            context_inputs,
            context_output,
        } => {
            let config = Config::read(&mut fs::File::open(
                cli.params_dir.join(&name_of_config(&cli.name)),
            )?)?;

            fs::create_dir_all(&output_dir)?;

            let public_inputs = parse_args(&public_inputs);
            let private_inputs = parse_args(&private_inputs);
            let context_inputs = parse_args(&context_inputs);

            match config.host_mode {
                HostMode::DEFAULT => {
                    config.dry_run::<DefaultHostEnvBuilder>(
                        &wasm_image,
                        &output_dir,
                        ExecutionArg {
                            public_inputs,
                            private_inputs,
                            context_inputs,
                            context_outputs: ContextOutput::default(),
                        },
                        context_output,
                    )?;
                }
                HostMode::STANDARD => {
                    config.dry_run::<StandardHostEnvBuilder>(
                        &wasm_image,
                        &output_dir,
                        delphinus_host::ExecutionArg {
                            public_inputs,
                            private_inputs,
                            context_inputs,
                            context_outputs: ContextOutput::default(),
                            indexed_witness: Rc::new(RefCell::new(HashMap::new())),
                            tree_db: None,
                        },
                        context_output,
                    )?;
                }
            }
        }

        Subcommands::Verify { output_dir } => {
            let config = Config::read(&mut fs::File::open(
                cli.params_dir.join(&name_of_config(&cli.name)),
            )?)?;

            config.verify(&cli.params_dir, &output_dir)?;
        }
    };

    Ok(())
}
