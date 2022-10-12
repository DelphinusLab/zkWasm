use std::borrow::Borrow;
use std::io::Write;
use std::{collections::HashMap, fmt, fs::File, io::Read, path::PathBuf};
use std::{env, fs};

use specs::types::Value;
use wasmi::{ExternVal, ExternVal::Func, ImportsBuilder, NopExternals};
use wast::kw::param;

// use crate::{
//     circuits::ZkWasmCircuitBuilder,
//     runtime::{WasmInterpreter, WasmRuntime},
// };
use crate::circuits::ZkWasmCircuitBuilder;
use crate::runtime::{WasmInterpreter, WasmRuntime};

#[derive(Debug, Clone)]
pub struct ArgumentError;

impl fmt::Display for ArgumentError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "invalid arguments for function")
    }
}

impl std::error::Error for ArgumentError {}

fn parser(f_sig: ExternVal, vv: Vec<&str>) -> Result<Vec<Value>, ArgumentError> {
    let f_sig = &f_sig.as_func().unwrap().signature();
    assert_eq!(f_sig.params().len(), vv.len());
    f_sig
        .params()
        .into_iter()
        .zip(vv.into_iter())
        .map(|(t, v)| match t {
            wasmi::ValueType::I32 => Ok(Value::I32(v.parse::<i32>().unwrap())),
            wasmi::ValueType::I64 => Ok(Value::I32(v.parse::<i32>().unwrap())),
            _ => Err(ArgumentError),
        })
        .collect::<Result<Vec<Value>, ArgumentError>>()
}

pub fn exec(
    file_path: &str,
    f_name: &str,
    vv: Vec<&str>,
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

    let compiler = WasmInterpreter::new();
    let compiled_module = compiler
        .compile(&binary, &ImportsBuilder::default(), &HashMap::new())
        .expect("file cannot be complied");

    let f_sig = compiled_module.instance.export_by_name(f_name).unwrap();

    let args = parser(f_sig, vv)?;
    let execution_log = compiler
        .run(&mut NopExternals, &compiled_module, f_name, args)
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

    let (params, vk, proof) = builder.bench_with_result();
    let mut params_fd = File::create(output_path.to_string() + "param.data").unwrap();
    params_fd.write_all(&params).unwrap();
    let mut vk_fd = File::create(output_path.to_string() + "vk.data").unwrap();
    vk_fd.write_all(&vk).unwrap();
    let mut proof_fd = File::create(output_path.to_string() + "proof.data").unwrap();
    proof_fd.write_all(&proof).unwrap();
    Ok(())
}
