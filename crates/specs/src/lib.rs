#![feature(trait_alias)]
#![deny(unused_imports)]
#![deny(dead_code)]

use std::env;
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

#[derive(Default, Clone)]
pub struct Tables {
    pub compilation_tables: CompilationTable,
    pub execution_tables: ExecutionTable,
}

impl Tables {
    pub fn write_json(&self, dir: Option<PathBuf>) {
        fn write_file(folder: &PathBuf, filename: &str, buf: &String) {
            let mut folder = folder.clone();
            folder.push(filename);
            let mut fd = std::fs::File::create(folder.as_path()).unwrap();
            folder.pop();

            fd.write(buf.as_bytes()).unwrap();
        }

        let itable = serde_json::to_string_pretty(&self.compilation_tables.itable).unwrap();
        let imtable = serde_json::to_string_pretty(&self.compilation_tables.imtable).unwrap();
        let etable = serde_json::to_string_pretty(&self.execution_tables.etable).unwrap();
        let external_host_call_table = serde_json::to_string_pretty(
            &self
                .execution_tables
                .etable
                .filter_external_host_call_table(),
        )
        .unwrap();
        let mtable = serde_json::to_string_pretty(&self.execution_tables.mtable).unwrap();
        let jtable = serde_json::to_string_pretty(&self.execution_tables.jtable).unwrap();

        let dir = dir.unwrap_or(env::current_dir().unwrap());
        write_file(&dir, "itable.json", &itable);
        write_file(&dir, "imtable.json", &imtable);
        write_file(&dir, "etable.json", &etable);
        write_file(&dir, "mtable.json", &mtable);
        write_file(&dir, "jtable.json", &jtable);
        write_file(&dir, "external_host_table.json", &external_host_call_table);
    }
}
