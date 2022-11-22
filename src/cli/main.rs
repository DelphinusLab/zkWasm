use clap::{arg, value_parser, Arg, ArgMatches};
use delphinus_zkwasm::{
    circuits::config::K,
    cli::{app_builder::AppBuilder, args::ArgBuilder, command::CommandBuilder},
};

fn parse_args(values: Vec<&str>) -> Vec<u64> {
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
                        .map(|x| u64::from_le_bytes(x.try_into().unwrap()))
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

struct SampleApp;

impl ArgBuilder for SampleApp {
    fn single_public_arg<'a>() -> Arg<'a> {
        arg!(--public [PUBLIC_INPUT] "Public arguments of your wasm program arguments of format value:type where type=i64|bytes|bytes-packed, multiple values should be separated with ','")
            .use_value_delimiter(true)
            .min_values(0)
            .value_parser(value_parser!(String))
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
        arg!(--private [PRIVATE_INPUT] "Private arguments of your wasm program arguments of format value:type where type=i64|bytes|bytes-packed, multiple values should be separated with ','")
            .use_value_delimiter(true)
            .min_values(0)
            .value_parser(value_parser!(String))
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

    const ZKWASM_K: u32 = K;
    const AGGREGATE_K: u32 = 23;
    const MAX_PUBLIC_INPUT_SIZE: usize = 1;

    const N_PROOFS: usize = 1;
}

/// Simple program to greet a person
fn main() {
    let app = SampleApp::app_builder();

    SampleApp::exec(app)
}
