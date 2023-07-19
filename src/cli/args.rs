use std::path::PathBuf;

use clap::arg;
use clap::value_parser;
use clap::Arg;
use clap::ArgMatches;

pub fn parse_args(values: Vec<&str>) -> Vec<u64> {
    values
        .into_iter()
        .map(|v| {
            let [v, t] = v.split(":").collect::<Vec<&str>>()[..] else { todo!() };
            match t {
                "i64" => {
                    if v.starts_with("0x") {
                        vec![
                            u64::from_str_radix(String::from(v).trim_start_matches("0x"), 16)
                                .unwrap(),
                        ]
                    } else {
                        vec![v.parse::<u64>().unwrap()]
                    }
                }
                "bytes" => {
                    if !v.starts_with("0x") {
                        panic!("bytes input need start with 0x");
                    }
                    let bytes = hex::decode(String::from(v).trim_start_matches("0x")).unwrap();
                    bytes
                        .into_iter()
                        .map(|x| u64::from(x))
                        .collect::<Vec<u64>>()
                }
                "bytes-packed" => {
                    if !v.starts_with("0x") {
                        panic!("bytes input need start with 0x");
                    }
                    let bytes = hex::decode(String::from(v).trim_start_matches("0x")).unwrap();
                    let bytes = bytes.chunks(8);
                    bytes
                        .into_iter()
                        .map(|x| {
                            let mut data = [0u8; 8];
                            data[..x.len()].copy_from_slice(x);

                            u64::from_le_bytes(data)
                        })
                        .collect::<Vec<u64>>()
                }

                _ => {
                    panic!("Unsupported input data type: {}", t)
                }
            }
        })
        .flatten()
        .collect()
}

pub trait ArgBuilder {
    fn zkwasm_k_arg<'a>() -> Arg<'a> {
        arg!(
            -k [K] "Circuit Size K"
        )
        .value_parser(value_parser!(u32))
    }
    fn parse_zkwasm_k_arg(matches: &ArgMatches) -> Option<u32> {
        matches.get_one("K").clone().map(|v| *v)
    }

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

    fn sol_dir_arg<'a>() -> Arg<'a> {
        arg!(
            -s --sol_dir [SOL_DIRECTORY] "Path of solidity directory."
        )
        .value_parser(value_parser!(PathBuf))
    }

    fn parse_sol_dir_arg(matches: &ArgMatches) -> PathBuf {
        matches
            .get_one::<PathBuf>("sol_dir")
            .map_or(PathBuf::from("sol"), |x| x.clone())
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

    fn single_instance_path_arg<'a>() -> Arg<'a> {
        arg!(
            -i --instance <INSTANCE_PATH> "Path of circuit instance."
        )
        .value_parser(value_parser!(PathBuf))
    }
    fn parse_single_instance_arg(matches: &ArgMatches) -> PathBuf {
        matches
            .get_one::<PathBuf>("instance")
            .expect("instance is required.")
            .clone()
    }

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

    fn dry_run_service_arg<'a>() -> Arg<'a> {
        arg!(
            -l --listen [LISTEN_PATH] "Execute dry-run as service."
        )
        .value_parser(value_parser!(PathBuf))
    }
    fn parse_dry_run_service_arg(matches: &ArgMatches) -> Option<PathBuf> {
        matches.get_one("listen").cloned()
    }
}
