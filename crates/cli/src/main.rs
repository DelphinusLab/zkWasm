#![deny(warnings)]
#![allow(clippy::too_many_arguments, clippy::while_let_on_iterator)]

use std::cell::RefCell;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::rc::Rc;

use anyhow::Result;
use app_builder::app;
use command::Subcommands;
use delphinus_host::StandardHostEnvBuilder;
use delphinus_zkwasm::runtime::host::default_env::DefaultHostEnvBuilder;
use delphinus_zkwasm::runtime::host::default_env::ExecutionArg;

use args::HostMode;
use config::Config;
use delphinus_zkwasm::runtime::host::HostEnvBuilder;
use file_backend::FileBackend;
use names::name_of_config;
use specs::args::parse_args;
use specs::slice_backend::memory::InMemoryBackend;
use specs::slice_backend::SliceBackend;

mod app_builder;
mod args;
mod command;
mod config;
mod file_backend;
mod names;

const TRIVIAL_WASM: &str = r#"
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

    match cli.subcommand {
        Subcommands::Setup(arg) => {
            let env_builder: Box<dyn HostEnvBuilder> = match arg.host_mode {
                HostMode::Default => Box::new(DefaultHostEnvBuilder::new(arg.k)),
                HostMode::Standard => Box::new(StandardHostEnvBuilder::new(arg.k)),
            };

            arg.setup(&*env_builder, &cli.name, &cli.params_dir)?;
        }
        Subcommands::DryRun(arg) => {
            let config = Config::read(&mut fs::File::open(
                cli.params_dir.join(name_of_config(&cli.name)),
            )?)?;

            let public_inputs = parse_args(&arg.running_arg.public_inputs);
            let private_inputs = parse_args(&arg.running_arg.private_inputs);
            let context_inputs = parse_args(&arg.running_arg.context_inputs);

            let env_builder: Box<dyn HostEnvBuilder> = match config.host_mode {
                HostMode::Default => Box::new(DefaultHostEnvBuilder::new(config.k)),
                HostMode::Standard => Box::new(StandardHostEnvBuilder::new(config.k)),
            };

            config.dry_run(
                &*env_builder,
                &arg.wasm_image,
                &arg.running_arg.output_dir,
                ExecutionArg {
                    public_inputs,
                    private_inputs,
                    context_inputs,
                    indexed_witness: Rc::new(RefCell::new(HashMap::default())),
                    tree_db: None,
                },
                arg.running_arg.context_output,
                arg.instruction_limit,
            )?;
        }
        Subcommands::Prove(arg) => {
            let trace_dir = arg.output_dir.join("traces");
            fs::create_dir_all(&trace_dir)?;

            let config = Config::read(&mut fs::File::open(
                cli.params_dir.join(name_of_config(&cli.name)),
            )?)?;

            let public_inputs = parse_args(&arg.running_arg.public_inputs);
            let private_inputs = parse_args(&arg.running_arg.private_inputs);
            let context_inputs = parse_args(&arg.running_arg.context_inputs);

            let trace_backend: Box<dyn SliceBackend> = if arg.file_backend {
                Box::new(FileBackend::new(cli.name, trace_dir))
            } else {
                Box::<InMemoryBackend>::default()
            };

            let env_builder: Box<dyn HostEnvBuilder> = match config.host_mode {
                HostMode::Default => Box::new(DefaultHostEnvBuilder::new(config.k)),
                HostMode::Standard => Box::new(StandardHostEnvBuilder::new(config.k)),
            };

            config.prove(
                &*env_builder,
                &arg.wasm_image,
                &cli.params_dir,
                &arg.output_dir,
                ExecutionArg {
                    public_inputs,
                    private_inputs,
                    context_inputs,
                    indexed_witness: Rc::new(RefCell::new(HashMap::default())),
                    tree_db: None,
                },
                arg.running_arg.context_output,
                arg.mock_test,
                trace_backend,
                arg.skip,
                arg.padding,
            )?;
        }
        Subcommands::Verify(arg) => {
            let config = Config::read(&mut fs::File::open(
                cli.params_dir.join(name_of_config(&cli.name)),
            )?)?;

            config.verify(&cli.params_dir, &arg.output_dir)?;
        }
    }

    Ok(())
}
