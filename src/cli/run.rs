use halo2_proofs::pairing::bn256::Fr as Fp;
use std::{
    fmt,
    fs::{self, File},
    io::{Read, Write},
    path::PathBuf,
};
use wasmi::{ExternVal, ImportsBuilder};

use crate::{
    circuits::ZkWasmCircuitBuilder,
    foreign::wasm_input_helper::runtime::register_wasm_input_foreign,
    runtime::{host::HostEnv, WasmInterpreter, WasmRuntime},
};

#[derive(Debug, Clone)]
pub struct ArgumentError;

impl fmt::Display for ArgumentError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "invalid arguments for function")
    }
}

impl std::error::Error for ArgumentError {}

fn check_sig(f_sig: &ExternVal) -> Result<(), ArgumentError> {
    let f_sig = &f_sig.as_func().unwrap().signature();

    // Pass arguments from function parameter is not support.
    if f_sig.params().len() != 0 {
        return Err(ArgumentError);
    }

    Ok(())
}

fn parse_args(values: Vec<&str>) -> Vec<u64> {
    values
        .into_iter()
        .map(|v| {
            let [v, t] = v.split(":").collect::<Vec<&str>>()[..] else { todo!() };
            match t {
                "i64" => {
                    if v.starts_with("0x") {
                        vec![u64::from_str_radix(String::from(v).trim_start_matches("0x"), 16).unwrap()]
                    } else {
                        vec![v.parse::<u64>().unwrap()]
                    }
                },
                "bytes" => {
                    if !v.starts_with("0x") {
                        panic!("bytes input need start with 0x");
                    }
                    let bytes = hex::decode(String::from(v).trim_start_matches("0x")).unwrap();
                    bytes.into_iter().map(|x| {u64::from(x)}).collect::<Vec<u64>>()
                },
                "bytes-packed" => {
                    if !v.starts_with("0x") {
                        panic!("bytes input need start with 0x");
                    }
                    let bytes = hex::decode(String::from(v).trim_start_matches("0x")).unwrap();
                    let bytes = bytes.chunks(8);
                    bytes.into_iter().map(|x| {u64::from_le_bytes(x.try_into().unwrap())}).collect::<Vec<u64>>()
                },

                _ => {
                    panic!("Unsupported input data type: {}", t)
                }
            }
        })
        .flatten()
        .collect()
}

pub fn exec(
    file_path: &str,
    f_name: &str,
    public_args: Vec<&str>,
    private_args: Vec<&str>,
    output_path: &str,
) -> Result<(), ArgumentError> {
    let mut binary = vec![];
    let path = PathBuf::from(file_path);

    println!(
        "read file from {}",
        fs::canonicalize(&path).unwrap().to_str().unwrap()
    );

    let mut f = File::open(path).unwrap();
    f.read_to_end(&mut binary).unwrap();

    let mut output_dir = std::env::current_dir().unwrap();
    output_dir.push(output_path);

    fs::create_dir(output_dir.clone()).unwrap();

    let public_inputs = parse_args(public_args);
    let private_inputs = parse_args(private_args);

    let mut env = HostEnv::new();
    register_wasm_input_foreign(&mut env, public_inputs.clone(), private_inputs.clone());
    let imports = ImportsBuilder::new().with_resolver("env", &env);

    let compiler = WasmInterpreter::new(env.function_plugin_lookup.clone());
    let compiled_module = compiler
        .compile(&binary, &imports)
        .expect("file cannot be complied");

    let f_sig = compiled_module.export_by_name(f_name).unwrap();
    check_sig(&f_sig)?;

    let _ = compiler
        .run(
            &mut env,
            &compiled_module,
            f_name,
            public_inputs.clone(),
            private_inputs,
        )
        .unwrap();

    let itable_str: Vec<String> = compiler
        .compile_table()
        .itable
        .iter()
        .map(|x| x.to_string())
        .collect();
    let imtable_str = compiler.compile_table().imtable.to_string();
    let etable_jtable_mtable_str = compiler.execution_tables().to_string();

    let serialize = |output_dir: &PathBuf, fname: &str, data:&[u8]| {
        let mut fd = File::create(output_dir.clone().to_str().unwrap().to_string() + fname).unwrap();
        fd.write_all(data).unwrap();
    };

    let mut i_fd = File::create(output_dir.clone().to_str().unwrap().to_string() + "/itable").unwrap();

    for data in &itable_str {
        i_fd.write_all(data.as_bytes()).unwrap();
    }


    let builder = ZkWasmCircuitBuilder::from_wasm_runtime(&compiler);

    let (params, vk, proof) = builder.run(
        public_inputs.into_iter().map(|v| Fp::from(v)).collect(),
        false,
    );

    serialize(&output_dir, "/imtable", imtable_str.as_bytes());
    serialize(&output_dir, "/ejmtable", etable_jtable_mtable_str.as_bytes());
    serialize(&output_dir, "/param.data", &params);
    serialize(&output_dir, "/vk.data", &vk);
    serialize(&output_dir, "/proof.data", &proof);

    Ok(())
}
