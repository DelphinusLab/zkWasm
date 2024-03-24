#![deny(warnings)]

use std::cell::RefCell;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::rc::Rc;

use anyhow::Result;
use app_builder::app;
use command::Subcommands;
use delphinus_host::StandardHostEnvBuilder;
use delphinus_zkwasm::foreign::context::ContextOutput;
use delphinus_zkwasm::runtime::host::default_env::DefaultHostEnvBuilder;
use delphinus_zkwasm::runtime::host::default_env::ExecutionArg;

use args::HostMode;
use specs::args::parse_args;

use crate::config::Config;
use crate::names::name_of_config;

mod app_builder;
mod args;
mod command;
mod config;
mod names;

const TRIVIAL_WASM: &'static str = r#"
(module
    (func (export "zkmain"))
)
"#;

#[derive(Debug)]
struct ZkWasmCli {
    name: String,
    params_dir: PathBuf,
    subcommand: Subcommands,
}

/// Simple program to greet a person
fn main() -> Result<()> {
    {
        env_logger::init();
    }

    let app = app();

    let cli: ZkWasmCli = app.get_matches().into();

    println!("{:?}", cli);

    match cli.subcommand {
        Subcommands::Setup(arg) => match arg.host_mode {
            HostMode::DEFAULT => {
                arg.setup::<DefaultHostEnvBuilder>(&cli.name, &cli.params_dir)?;
            }
            HostMode::STANDARD => {
                arg.setup::<StandardHostEnvBuilder>(&cli.name, &cli.params_dir)?;
            }
        },
        Subcommands::DryRun(arg) => {
            let config = Config::read(&mut fs::File::open(
                cli.params_dir.join(&name_of_config(&cli.name)),
            )?)?;

            let public_inputs = parse_args(&arg.running_arg.public_inputs);
            let private_inputs = parse_args(&arg.running_arg.private_inputs);
            let context_inputs = parse_args(&arg.running_arg.context_inputs);

            match config.host_mode {
                HostMode::DEFAULT => {
                    config.dry_run::<DefaultHostEnvBuilder>(
                        &arg.wasm_image,
                        &arg.running_arg.output_dir,
                        ExecutionArg {
                            public_inputs,
                            private_inputs,
                            context_inputs,
                            context_outputs: ContextOutput::default(),
                        },
                        arg.running_arg.context_output,
                    )?;
                }
                HostMode::STANDARD => {
                    config.dry_run::<StandardHostEnvBuilder>(
                        &arg.wasm_image,
                        &arg.running_arg.output_dir,
                        delphinus_host::ExecutionArg {
                            public_inputs,
                            private_inputs,
                            context_inputs,
                            context_outputs: ContextOutput::default(),
                            indexed_witness: Rc::new(RefCell::new(HashMap::new())),
                            tree_db: None,
                        },
                        arg.running_arg.context_output,
                    )?;
                }
            }
        }
        Subcommands::Prove(arg) => {
            let config = Config::read(&mut fs::File::open(
                cli.params_dir.join(&name_of_config(&cli.name)),
            )?)?;

            let public_inputs = parse_args(&arg.running_arg.public_inputs);
            let private_inputs = parse_args(&arg.running_arg.private_inputs);
            let context_inputs = parse_args(&arg.running_arg.context_inputs);

            match config.host_mode {
                HostMode::DEFAULT => {
                    config.prove::<DefaultHostEnvBuilder>(
                        &arg.wasm_image,
                        &cli.params_dir,
                        &arg.output_dir,
                        ExecutionArg {
                            public_inputs,
                            private_inputs,
                            context_inputs,
                            context_outputs: ContextOutput::default(),
                        },
                        arg.running_arg.context_output,
                        arg.mock_test,
                    )?;
                }
                HostMode::STANDARD => {
                    config.prove::<StandardHostEnvBuilder>(
                        &arg.wasm_image,
                        &cli.params_dir,
                        &arg.output_dir,
                        delphinus_host::ExecutionArg {
                            public_inputs,
                            private_inputs,
                            context_inputs,
                            context_outputs: ContextOutput::default(),
                            indexed_witness: Rc::new(RefCell::new(HashMap::new())),
                            tree_db: None,
                        },
                        arg.running_arg.context_output,
                        arg.mock_test,
                    )?;
                }
            }
        }
        Subcommands::Verify(arg) => {
            let config = Config::read(&mut fs::File::open(
                cli.params_dir.join(&name_of_config(&cli.name)),
            )?)?;

            match config.host_mode {
                HostMode::DEFAULT => {
                    config.verify::<StandardHostEnvBuilder>(&cli.params_dir, &arg.output_dir)?
                }
                HostMode::STANDARD => {
                    config.verify::<StandardHostEnvBuilder>(&cli.params_dir, &arg.output_dir)?
                }
            }
        }
    }

    Ok(())
}
