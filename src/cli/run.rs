use halo2_proofs::pairing::bn256::Fr as Fp;
use std::fs;
use std::io::Write;
use std::process::exit;
use std::{fmt, fs::File, io::Read, path::PathBuf};
use wasmi::{ExternVal, ImportsBuilder};
use specs::{host_function::HostPlugin, types::ValueType};

use crate::circuits::ZkWasmCircuitBuilder;
use crate::foreign::wasm_input_helper::runtime::register_wasm_input_foreign;
use crate::runtime::host::{ForeignContext, HostEnv};
use crate::runtime::{WasmInterpreter, WasmRuntime};
struct Context {}
impl ForeignContext for Context {}
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
        .map(|v| v.parse::<u64>().unwrap())
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

    fs::create_dir(PathBuf::from(output_path)).unwrap();

    let public_inputs = parse_args(public_args);
    let private_inputs = parse_args(private_args);

    let mut env = HostEnv::new();
    env.register_function("abort",
    0, Box::new(Context {}),
    specs::host_function::Signature {
        params: vec![ValueType::I32, ValueType::I32, ValueType::I32, ValueType::I32],
        return_type: None,
    },
    Box::new(|_, args| {
        exit(-1);
    }), HostPlugin::AssemblyScriptPolyfill);
    register_wasm_input_foreign(&mut env, public_inputs.clone(), private_inputs.clone());
    let imports = ImportsBuilder::new().with_resolver("env", &env);

    let compiler = WasmInterpreter::new();
    let compiled_module = compiler
        .compile(&binary, &imports, &env.function_plugin_lookup)
        .expect("file cannot be complied");

    let f_sig = compiled_module.instance.export_by_name(f_name).unwrap();
    check_sig(&f_sig)?;

    let execution_log = compiler
        .run(
            &mut env,
            &compiled_module,
            f_name,
            public_inputs.clone(),
            private_inputs,
        )
        .unwrap();

    let itable_str: Vec<String> = compiled_module
        .tables
        .itable
        .iter()
        .map(|x| x.to_string())
        .collect();
    let imtable_str = compiled_module.tables.imtable.to_string();
    let etable_jtable_mtable_str = execution_log.tables.to_string();

    let mut i_fd = File::create(output_path.to_string() + "itable").unwrap();

    for data in &itable_str {
        i_fd.write_all(data.as_bytes()).unwrap();
    }

    let mut im_fd = File::create(output_path.to_string() + "imtable").unwrap();
    im_fd.write_all(imtable_str.as_bytes()).unwrap();
    let mut ejm_fd = File::create(output_path.to_string() + "etable_jtable_mtable").unwrap();
    ejm_fd
        .write_all(etable_jtable_mtable_str.as_bytes())
        .unwrap();

    let builder = ZkWasmCircuitBuilder {
        compile_tables: compiled_module.tables,
        execution_tables: execution_log.tables,
    };

    let (params, vk, proof) =
        builder.bench_with_result(public_inputs.into_iter().map(|v| Fp::from(v)).collect());

    let mut params_fd = File::create(output_path.to_string() + "param.data").unwrap();
    params_fd.write_all(&params).unwrap();
    let mut vk_fd = File::create(output_path.to_string() + "vk.data").unwrap();
    vk_fd.write_all(&vk).unwrap();
    let mut proof_fd = File::create(output_path.to_string() + "proof.data").unwrap();
    proof_fd.write_all(&proof).unwrap();

    Ok(())
}
