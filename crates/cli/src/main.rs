use anyhow::Result;
use app_builder::AppBuilder;
use args::parse_args;
use args::ArgBuilder;
use clap::value_parser;
use clap::Arg;
use clap::ArgAction;
use clap::ArgMatches;
use command::CommandBuilder;

pub mod app_builder;
pub mod args;
pub mod command;
pub mod exec;

struct SampleApp;

impl ArgBuilder for SampleApp {
    fn single_public_arg<'a>() -> Arg<'a> {
        Arg::new("public")
            .long("public")
            .value_parser(value_parser!(String))
            .action(ArgAction::Append)
            .help("Public arguments of your wasm program arguments of format value:type where type=i64|bytes|bytes-packed")
            .min_values(0)
    }
    fn parse_single_public_arg(matches: &ArgMatches) -> Vec<u64> {
        let inputs: Vec<&str> = matches
            .get_many("public")
            .unwrap_or_default()
            .map(|v: &String| v.as_str())
            .collect();

        parse_args(inputs.into())
    }

    fn aggregate_public_args<'a>() -> Arg<'a> {
        // We only aggregate one proof in the sample program.
        Self::single_public_arg()
    }
    fn parse_aggregate_public_args(matches: &ArgMatches) -> Vec<Vec<u64>> {
        let inputs = Self::parse_single_public_arg(matches);

        vec![inputs]
    }

    fn single_private_arg<'a>() -> Arg<'a> {
        Arg::new("private")
            .long("private")
            .value_parser(value_parser!(String))
            .action(ArgAction::Append)
            .help("Private arguments of your wasm program arguments of format value:type where type=i64|bytes|bytes-packed")
            .min_values(0)
    }
    fn parse_single_private_arg(matches: &ArgMatches) -> Vec<u64> {
        let inputs: Vec<&str> = matches
            .get_many("private")
            .unwrap_or_default()
            .map(|v: &String| v.as_str())
            .collect();

        parse_args(inputs.into())
    }

    fn aggregate_private_args<'a>() -> Arg<'a> {
        // We only aggregate one proof in the sample program.
        Self::single_private_arg()
    }
    fn parse_aggregate_private_args(matches: &ArgMatches) -> Vec<Vec<u64>> {
        let inputs = Self::parse_single_private_arg(matches);

        vec![inputs]
    }
}
impl CommandBuilder for SampleApp {}
impl AppBuilder for SampleApp {
    const NAME: &'static str = "zkwasm";
    const VERSION: &'static str = "v1.0-beta";

    #[cfg(feature = "checksum")]
    const AGGREGATE_K: u32 = 22;

    #[cfg(not(feature = "checksum"))]
    const AGGREGATE_K: u32 = 22;

    const MAX_PUBLIC_INPUT_SIZE: usize = 64;

    const N_PROOFS: usize = 1;
}

/// Simple program to greet a person
fn main() -> Result<()> {
    let app = SampleApp::app_builder();

    SampleApp::exec(app)
}
