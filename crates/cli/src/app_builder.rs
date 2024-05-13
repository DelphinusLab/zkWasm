use std::path::PathBuf;

use clap::arg;
use clap::command;
use clap::value_parser;
use clap::App;
use clap::Arg;
use clap::ArgAction;
use clap::ArgMatches;
use clap::Command;
use clap::ValueHint;

use crate::args::HostMode;
use crate::command::DryRunArg;
use crate::command::ProveArg;
use crate::command::RunningArg;
use crate::command::SetupArg;
use crate::command::Subcommands;
use crate::command::VerifyArg;
use crate::ZkWasmCli;

trait ArgBuilder<T> {
    fn builder() -> Arg<'static>;
    fn parse(matches: &ArgMatches) -> T;
}

struct WasmImageArg;
impl ArgBuilder<Option<PathBuf>> for WasmImageArg {
    fn builder() -> Arg<'static> {
        arg!(
            --wasm <WASM> "Path to the Wasm image"
        )
        .value_parser(value_parser!(PathBuf))
        .value_hint(ValueHint::FilePath)
    }

    fn parse(matches: &ArgMatches) -> Option<PathBuf> {
        matches.get_one::<PathBuf>("wasm").cloned()
    }
}

struct OutputDirArg;
impl ArgBuilder<PathBuf> for OutputDirArg {
    fn builder() -> Arg<'static> {
        arg!(-o --output <OUTPUT> "Path to output directory")
            .value_parser(value_parser!(PathBuf))
            .value_hint(ValueHint::DirPath)
    }

    fn parse(matches: &ArgMatches) -> PathBuf {
        matches.get_one::<PathBuf>("output").cloned().unwrap()
    }
}

struct PublicInputsArg;
impl ArgBuilder<Vec<String>> for PublicInputsArg {
    fn builder() -> Arg<'static> {
        arg!(--public <PUBLIC_INPUT> ... "Public inputs with format value:type where type=i64|bytes|bytes-packed, values can be separated by `,` or multiple occurrence of `--public`")
     .takes_value(true).value_delimiter(',').required(false)
    }

    fn parse(matches: &ArgMatches) -> Vec<String> {
        matches
            .get_many::<String>("public")
            .unwrap_or_default()
            .map(|s| s.to_string())
            .collect()
    }
}

struct PrivateInputsArg;
impl ArgBuilder<Vec<String>> for PrivateInputsArg {
    fn builder() -> Arg<'static> {
        arg!(--private <PRIVATE_INPUT> ... "Private inputs with format value:type where type=i64|bytes|bytes-packed, values can be separated by `,` or multiple occurrence of `--private`")
       .takes_value(true).value_delimiter(',').required(false)
    }

    fn parse(matches: &ArgMatches) -> Vec<String> {
        matches
            .get_many::<String>("private")
            .unwrap_or_default()
            .map(|s| s.to_string())
            .collect()
    }
}

struct ContextInputsArg;
impl ArgBuilder<Vec<String>> for ContextInputsArg {
    fn builder() -> Arg<'static> {
        arg!(--ctxin <CONTEXT_INPUT> ... "Context inputs with format value:type where type=i64|bytes|bytes-packed, values can be separated by `,` or multiple occurrence of `--ctxin`")
       .takes_value(true).value_delimiter(',').required(false)
    }

    fn parse(matches: &ArgMatches) -> Vec<String> {
        matches
            .get_many::<String>("ctxin")
            .unwrap_or_default()
            .map(|s| s.to_string())
            .collect()
    }
}

struct ContextOutputArg;
impl ArgBuilder<Option<String>> for ContextOutputArg {
    fn builder() -> Arg<'static> {
        arg!(--ctxout [CONTEXT_OUTPUT] "Path to context output")
    }

    fn parse(matches: &ArgMatches) -> Option<String> {
        matches.get_one("ctxout").cloned()
    }
}

struct FileBackendArg;
impl ArgBuilder<bool> for FileBackendArg {
    fn builder() -> Arg<'static> {
        arg!(--file "Enabling the file backend for table to support enormous execution trace. It may reduce the speed of execution.")
            .action(ArgAction::SetTrue)
    }

    fn parse(matches: &ArgMatches) -> bool {
        matches.get_flag("file")
    }
}

struct MockTestArg;
impl ArgBuilder<bool> for MockTestArg {
    fn builder() -> Arg<'static> {
        arg!(-m --mock "Enable mock test before proving").action(ArgAction::SetTrue)
    }

    fn parse(matches: &ArgMatches) -> bool {
        matches.get_flag("mock")
    }
}

fn setup_command() -> Command<'static> {
    let command = Command::new("setup")
        .about("Setup a new zkWasm circuit for provided Wasm image")
        .arg(
            arg!(-k <K> "Size of the circuit.")
                .default_value(if cfg!(feature = "continuation") {
                    "22"
                } else {
                    "18"
                })
                .value_parser(value_parser!(u32).range(18..))
                .required(false),
        )
        .arg(
            arg!(
                --host <HOST_MODE> "Specify execution host environment for the runtime"
            )
            .default_value("default")
            .value_parser(value_parser!(HostMode))
            .required(false),
        )
        .arg(
            arg!(
                --phantom <PHANTOM_FUNCTIONS> "Specify phantom functions whose body will be ignored in the circuit"
            ).takes_value(true)
            .value_delimiter(',')
            .required(false)
        );

    let command = if cfg!(not(feature = "uniform-circuit")) {
        command.arg(WasmImageArg::builder())
    } else {
        command
    };

    command
}

fn dry_run_command() -> Command<'static> {
    Command::new("dry-run")
        .about("Execute the Wasm image without generating a proof")
        .arg(WasmImageArg::builder())
        .arg(PublicInputsArg::builder())
        .arg(PrivateInputsArg::builder())
        .arg(ContextInputsArg::builder())
        .arg(ContextOutputArg::builder())
        .arg(OutputDirArg::builder())
}

fn prove_command() -> Command<'static> {
    Command::new("prove")
        .about("Execute the Wasm image and generate a proof")
        .arg(WasmImageArg::builder())
        .arg(PublicInputsArg::builder())
        .arg(PrivateInputsArg::builder())
        .arg(ContextInputsArg::builder())
        .arg(ContextOutputArg::builder())
        .arg(OutputDirArg::builder())
        .arg(MockTestArg::builder())
        .arg(FileBackendArg::builder())
}

fn verify_command() -> Command<'static> {
    Command::new("verify")
        .about("Verify the proof")
        .arg(OutputDirArg::builder())
}

pub(crate) fn app() -> App<'static> {
    command!()
        .author("delphinus-lab")
        .arg(arg!(<NAME> "Name of the configuration."))
        .arg(
            arg!(
                --params <PARAMS> "Directory to setup params and configuration."
            )
            .value_parser(value_parser!(PathBuf)),
        )
        .subcommand(setup_command())
        .subcommand(dry_run_command())
        .subcommand(prove_command())
        .subcommand(verify_command())
        .subcommand_required(true)
}

impl Into<SetupArg> for &ArgMatches {
    fn into(self) -> SetupArg {
        SetupArg {
            k: *self.get_one::<u32>("K").unwrap(),
            host_mode: *self.get_one::<HostMode>("host").unwrap(),
            phantom_functions: self
                .get_many::<String>("phantom")
                .unwrap_or_default()
                .map(|v| v.to_string())
                .collect::<Vec<_>>(),
            wasm_image: WasmImageArg::parse(self),
        }
    }
}

impl Into<RunningArg> for &ArgMatches {
    fn into(self) -> RunningArg {
        RunningArg {
            output_dir: OutputDirArg::parse(self),
            public_inputs: PublicInputsArg::parse(self),
            private_inputs: PrivateInputsArg::parse(self),
            context_inputs: ContextInputsArg::parse(self),
            context_output: ContextOutputArg::parse(self),
        }
    }
}

impl Into<DryRunArg> for &ArgMatches {
    fn into(self) -> DryRunArg {
        DryRunArg {
            wasm_image: WasmImageArg::parse(self).unwrap(),
            running_arg: self.into(),
        }
    }
}

impl Into<ProveArg> for &ArgMatches {
    fn into(self) -> ProveArg {
        ProveArg {
            wasm_image: WasmImageArg::parse(self).unwrap(),
            output_dir: OutputDirArg::parse(self),
            running_arg: self.into(),
            mock_test: MockTestArg::parse(self),
            file_backend: FileBackendArg::parse(self),
        }
    }
}

impl Into<VerifyArg> for &ArgMatches {
    fn into(self) -> VerifyArg {
        VerifyArg {
            output_dir: OutputDirArg::parse(self),
        }
    }
}

impl Into<ZkWasmCli> for ArgMatches {
    fn into(self) -> ZkWasmCli {
        let subcommand = match self.subcommand() {
            Some(("setup", sub_matches)) => Subcommands::Setup(sub_matches.into()),
            Some(("dry-run", sub_matches)) => Subcommands::DryRun(sub_matches.into()),
            Some(("prove", sub_matches)) => Subcommands::Prove(sub_matches.into()),
            Some(("verify", sub_matches)) => Subcommands::Verify(sub_matches.into()),
            _ => unreachable!("unknown subcommand"),
        };

        ZkWasmCli {
            name: self.get_one::<String>("NAME").unwrap().to_owned(),
            params_dir: self.get_one::<PathBuf>("params").unwrap().to_owned(),
            subcommand,
        }
    }
}
