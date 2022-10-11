use std::borrow::Borrow;
use std::io::Write;
use std::{collections::HashMap, fmt, fs::File, io::Read, path::PathBuf};
use std::{env, fs};

use specs::types::Value;
use wasmi::{ImportsBuilder, NopExternals};

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
        write!(f, "mismatch type number and argument number")
    }
}

impl std::error::Error for ArgumentError {}

pub fn arg_parser(tv: Vec<&str>, vv: Vec<&str>) -> Result<Vec<Value>, ArgumentError> {
    if tv.len() != vv.len() {
        Err(ArgumentError)
    } else {
        let result = tv
            .iter()
            .zip(vv.iter())
            .map(|(&tStr, &vStr)| match tStr {
                "i32" => Value::I32(vStr.parse::<i32>().unwrap()),
                "i64" => Value::I64(vStr.parse::<i64>().unwrap()),
                _ => {
                    unimplemented!();
                }
            })
            .collect();

        Ok(result)
    }
}

pub fn exec(
    file_path: &str,
    f_name: &str,
    tv: Vec<&str>,
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

    fs::create_dir(PathBuf::from(output_path));

    let compiler = WasmInterpreter::new();
    let compiled_module = compiler
        .compile(&binary, &ImportsBuilder::default(), &HashMap::new())
        .expect("file cannot be complied");
    let args = arg_parser(tv, vv)?;
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
