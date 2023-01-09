use std::path::PathBuf;

use clap::{arg, value_parser, Arg, ArgMatches};

pub trait ArgBuilder {
    fn zkwasm_file_arg<'a>() -> Arg<'a> {
        arg!(
            -w --wasm <WASM_BINARY> "Path of the Wasm binary file"
        )
        .value_parser(value_parser!(PathBuf))
    }
    fn parse_zkwasm_file_arg(matches: &ArgMatches) -> PathBuf {
        matches
            .get_one::<PathBuf>("wasm")
            .expect("wasm is required.")
            .clone()
    }

    fn function_name_arg<'a>() -> Arg<'a> {
        arg!(
            -f --function <FUNCTION_NAME> "Function you would like to run."
        )
    }
    fn parse_function_name(matches: &ArgMatches) -> String {
        matches
            .get_one::<String>("function")
            .expect("function is required")
            .to_string()
    }

    fn output_path_arg<'a>() -> Arg<'a> {
        arg!(
            -o --output [OUTPUT_PATH] "Path of the output files.\nThe md5 of the wasm binary file is the default path if not supplied."
        ).value_parser(value_parser!(PathBuf))
    }

    fn proof_path_arg<'a>() -> Arg<'a> {
        arg!(
            -p --proof <PROOF_PATH> "Path of proof."
        )
        .value_parser(value_parser!(PathBuf))
    }

    fn parse_proof_path_arg(matches: &ArgMatches) -> PathBuf {
        matches
            .get_one::<PathBuf>("proof")
            .expect("proof is required.")
            .clone()
    }

    fn template_path_arg<'a>() -> Arg<'a> {
        arg!(
            -t --template <PROOF_PATH> "Path of solidity template."
        )
        .value_parser(value_parser!(PathBuf))
    }

    fn parse_template_path_arg(matches: &ArgMatches) -> Option<PathBuf> {
        matches
            .get_one::<PathBuf>("template")
            .map_or(None, |x| Some (x.clone()))
    }


    fn auxonly_arg<'a>() -> Arg<'a> {
        arg!(
            -a --auxonly "Generate aux file only."
        )
        .takes_value(false)
    }


    fn single_public_arg<'a>() -> Arg<'a>;
    fn parse_single_public_arg(matches: &ArgMatches) -> Vec<u64>;

    fn aggregate_public_args<'a>() -> Arg<'a>;
    fn parse_aggregate_public_args(matches: &ArgMatches) -> Vec<Vec<u64>>;

    fn single_private_arg<'a>() -> Arg<'a>;
    fn parse_single_private_arg(matches: &ArgMatches) -> Vec<u64>;

    fn aggregate_private_args<'a>() -> Arg<'a>;
    fn parse_aggregate_private_args(matches: &ArgMatches) -> Vec<Vec<u64>>;

    fn instances_path_arg<'a>() -> Arg<'a> {
        arg!(
            -i --instances <AGGREGATE_INSTANCE_PATH> "Path of aggregate instances."
        )
        .value_parser(value_parser!(PathBuf))
    }
    fn parse_aggregate_instance(matches: &ArgMatches) -> PathBuf {
        matches
            .get_one::<PathBuf>("instances")
            .expect("instances is required.")
            .clone()
    }
    fn parse_auxonly(matches: &ArgMatches) -> bool {
        matches
            .get_many::<String>("auxonly")
            .map_or(false, |_| true)
    }

}
