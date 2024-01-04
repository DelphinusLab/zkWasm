#![feature(trait_alias)]
#![deny(unused_imports)]
#![deny(dead_code)]

use std::env;
use std::io::BufReader;
use std::io::Write;
use std::path::PathBuf;

use brtable::ElemTable;
use configure_table::ConfigureTable;
use etable::EventTable;
use imtable::InitMemoryTable;
use itable::InstructionTable;
use jtable::JumpTable;
use jtable::StaticFrameEntry;
use mtable::MTable;
use serde::Deserialize;
use serde::Serialize;
use serde::de::DeserializeOwned;

#[macro_use]
extern crate lazy_static;

pub mod args;
pub mod brtable;
pub mod configure_table;
pub mod encode;
pub mod etable;
pub mod external_host_call_table;
pub mod host_function;
pub mod imtable;
pub mod itable;
pub mod jtable;
pub mod mtable;
pub mod step;
pub mod types;

#[derive(Default, Serialize, Deserialize, Debug, Clone)]
pub struct CompilationTable {
    pub itable: InstructionTable,
    pub imtable: InitMemoryTable,
    pub elem_table: ElemTable,
    pub configure_table: ConfigureTable,
    pub static_jtable: Vec<StaticFrameEntry>,
    pub fid_of_entry: u32,
}

#[derive(Default, Serialize, Deserialize, Clone)]
pub struct ExecutionTable {
    pub etable: EventTable,
    pub mtable: MTable,
    pub jtable: JumpTable,
}

pub enum FileType {
    JSON,
    FLEXBUFFERS,
}

#[derive(Default, Clone)]
pub struct Tables {
    pub compilation_tables: CompilationTable,
    pub execution_tables: ExecutionTable
}

impl Tables {
    pub fn write_json(&self, dir: Option<PathBuf>, public_inputs_and_outputs: &Vec<u64>) {
        fn write_file(folder: &PathBuf, filename: &str, buf: String) {
            let mut folder = folder.clone();
            folder.push(filename);
            let mut fd = std::fs::File::create(folder.as_path()).unwrap();
            folder.pop();

            fd.write(buf.as_bytes()).unwrap();
        }

        let dir = dir.unwrap_or(env::current_dir().unwrap());

        let compilation_table =
        serde_json::to_string_pretty(&self.compilation_tables).unwrap();
        let execution_table = serde_json::to_string_pretty(&self.execution_tables).unwrap();
        let external_host_call_table = serde_json::to_string_pretty(
            &self
                .execution_tables
                .etable
                .filter_external_host_call_table(),
        ).unwrap();
        let instances = serde_json::to_string_pretty(&public_inputs_and_outputs).unwrap();
        write_file(&dir, "compilation.json", compilation_table);
        write_file(&dir, "execution.json", execution_table);
        write_file(&dir, "instance.json", instances);
        write_file(&dir, "external_host_table.json", external_host_call_table);
    }

    pub fn load_table(dir: PathBuf) -> (Tables, Vec<u64>) {
        fn load_file<T:DeserializeOwned>(folder: &PathBuf, filename: &str) -> T {
            let mut folder = folder.clone();
            folder.push(filename);
            let file = std::fs::File::open(folder.as_path()).unwrap();
            let reader = BufReader::new(file);
            serde_json::from_reader(reader).unwrap()
        }
        let compilation_tables: CompilationTable = load_file(&dir, "compilation.json");
        let execution_tables: ExecutionTable = load_file(&dir, "execution.json");
        let public_inputs_and_outputs: Vec<u64> = load_file(&dir, "instance.json");
        (Tables { compilation_tables, execution_tables}, public_inputs_and_outputs)
    }
}
