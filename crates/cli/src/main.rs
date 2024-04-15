#![deny(warnings)]

use std::fs;
use std::fs::File;
use std::path::PathBuf;

use anyhow::Result;
use app_builder::app;
use command::Subcommands;
use delphinus_host::StandardHostEnvBuilder;
use delphinus_zkwasm::runtime::host::default_env::DefaultHostEnvBuilder;
use delphinus_zkwasm::runtime::host::default_env::ExecutionArg;

use args::HostMode;
use config::Config;
use delphinus_zkwasm::runtime::host::HostEnvBuilder;
use names::name_of_config;
use names::name_of_etable_slice;
use specs::args::parse_args;
use specs::TraceBackend;

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

    match cli.subcommand {
        Subcommands::Setup(arg) => {
            let env_builder: Box<dyn HostEnvBuilder> = match arg.host_mode {
                HostMode::DEFAULT => Box::new(DefaultHostEnvBuilder),
                HostMode::STANDARD => Box::new(StandardHostEnvBuilder::default()),
            };

            arg.setup(&env_builder, &cli.name, &cli.params_dir)?;
        }
        Subcommands::DryRun(arg) => {
            let config = Config::read(&mut fs::File::open(
                cli.params_dir.join(&name_of_config(&cli.name)),
            )?)?;

            let public_inputs = parse_args(&arg.running_arg.public_inputs);
            let private_inputs = parse_args(&arg.running_arg.private_inputs);
            let context_inputs = parse_args(&arg.running_arg.context_inputs);

            let env_builder: Box<dyn HostEnvBuilder> = match config.host_mode {
                HostMode::DEFAULT => Box::new(DefaultHostEnvBuilder),
                HostMode::STANDARD => Box::new(StandardHostEnvBuilder::default()),
            };

            config.dry_run(
                &env_builder,
                &arg.wasm_image,
                &arg.running_arg.output_dir,
                ExecutionArg {
                    public_inputs,
                    private_inputs,
                    context_inputs,
                },
                arg.running_arg.context_output,
            )?;
        }
        Subcommands::Prove(arg) => {
            let trace_dir = arg.output_dir.clone().join("traces");
            fs::create_dir_all(&trace_dir)?;

            let config = Config::read(&mut fs::File::open(
                cli.params_dir.join(&name_of_config(&cli.name)),
            )?)?;

            let public_inputs = parse_args(&arg.running_arg.public_inputs);
            let private_inputs = parse_args(&arg.running_arg.private_inputs);
            let context_inputs = parse_args(&arg.running_arg.context_inputs);

            let file_backend = arg.file_backend;
            let backend = if file_backend {
                TraceBackend::File(Box::new(move |slice, etable| {
                    let filename_of_etable_slice =
                        PathBuf::from(name_of_etable_slice(&cli.name, slice));
                    let path = trace_dir.join(&filename_of_etable_slice);

                    let mut fd = File::create(&path).unwrap();
                    serde_json::to_writer(&mut fd, etable).unwrap();

                    path
                }))
            } else {
                TraceBackend::Memory
            };

            let env_builder: Box<dyn HostEnvBuilder> = match config.host_mode {
                HostMode::DEFAULT => Box::new(DefaultHostEnvBuilder),
                HostMode::STANDARD => Box::new(StandardHostEnvBuilder::default()),
            };

            config.prove(
                &env_builder,
                &arg.wasm_image,
                &cli.params_dir,
                &arg.output_dir,
                ExecutionArg {
                    public_inputs,
                    private_inputs,
                    context_inputs,
                },
                arg.running_arg.context_output,
                arg.mock_test,
                backend,
            )?;
        }
        Subcommands::Verify(arg) => {
            let config = Config::read(&mut fs::File::open(
                cli.params_dir.join(&name_of_config(&cli.name)),
            )?)?;

            config.verify(&cli.params_dir, &arg.output_dir)?;
        }
    }

    Ok(())
}
