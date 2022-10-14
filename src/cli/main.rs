use clap::{value_parser, App,AppSettings, Arg, SubCommand};

use delphinus_zkwasm::cli::run;

/// Simple program to greet a person
fn main() {
    let wasm_file_arg = Arg::with_name("wasm_file")
        .short('w')
        .long("wasm")
        .value_name("FILEPATH")
        .help("Path of the wasm file")
        .required(true)
        .takes_value(true)
        .value_parser(value_parser!(std::string::String));
    let fn_name_arg = Arg::with_name("function_name")
        .long("fname")
        .short('f')
        .required(true)
        .value_name("FUNCTIONNAME")
        .help("Function you would like to run from the file")
        .takes_value(true)
        .value_parser(value_parser!(std::string::String));

    let public_value_arg = Arg::with_name("public_args")
        .long("public")
        .value_name("PUBLIC ARGUMENTS")
        .help("Public arguments of your wasm program arguments, multiple values should be separated with ','")
        .required(false)
        .takes_value(true)
        .use_delimiter(true)
        .value_delimiter(',')
        .min_values(0)
        .value_parser(value_parser!(std::string::String));

    let private_value_arg = Arg::with_name("private_args")
        .long("private")
        .value_name("PRIVATE ARGUMENTS")
        .help("Private arguments of your wasm program arguments, multiple values should be separated with ','")
        .required(false)
        .takes_value(true)
        .use_delimiter(true)
        .value_delimiter(',')
        .min_values(0)
        .value_parser(value_parser!(std::string::String));

    let output_path = Arg::with_name("output_path")
        .short('o')
        .long("output")
        .value_name("OUTPUTPATH")
        .help("Path of the output files default: './output/'")
        .required(false)
        .takes_value(true)
        .value_parser(value_parser!(std::string::String));

    let app = App::new("zkwasm")
        .setting(AppSettings::SubcommandRequired)
        .version("v1.0-beta")
        .subcommand(
            SubCommand::with_name("run")
            .about("Run your function from your wasm program with inputs.\nType 'cli run --help' for more information\nOnly support I32 type now")
                .arg(wasm_file_arg)
                .arg(public_value_arg)
                .arg(private_value_arg)
                .arg(fn_name_arg)
                .arg(output_path),
        )
        .get_matches();

    match app.subcommand() {
        Some(("run", m)) => {
            let wasm_file: &str = m.value_of("wasm_file").unwrap();
            let fn_name: &str = m.value_of("function_name").unwrap();
            let public_inputs: Vec<&str> = m.values_of("public_args").unwrap().collect();
            let private_inputs: Vec<&str> = m.values_of("private_args").unwrap().collect();
            let output_path: &str = m.value_of("output_path").unwrap_or("./output/");
            run::exec(
                wasm_file,
                fn_name,
                public_inputs,
                private_inputs,
                output_path,
            )
            .unwrap();
        }
        _ => unimplemented!(),
    };
}
