use std::{env, io::Write, path::PathBuf};

use etable::EventTable;
use imtable::InitMemoryTable;
use mtable::MTable;
use serde::Serialize;

use self::{itable::InstructionTableEntry, jtable::JumpTableEntry};

pub mod etable;
pub mod host_function;
pub mod imtable;
pub mod itable;
pub mod jtable;
pub mod mtable;
pub mod step;
pub mod types;

#[derive(Default, Serialize, Debug, Clone)]
pub struct CompileTable {
    pub itable: Vec<InstructionTableEntry>,
    pub imtable: InitMemoryTable,
}

impl CompileTable {
    pub fn to_string(&self) -> String {
        serde_json::to_string(self).unwrap()
    }
}

#[derive(Default, Serialize, Clone)]
pub struct ExecutionTable {
    pub etable: EventTable,
    pub mtable: MTable,
    pub jtable: Vec<JumpTableEntry>,
}

impl ExecutionTable {
    pub fn to_string(&self) -> String {
        serde_json::to_string(self).unwrap()
    }
}

fn write_file(folder: &PathBuf, filename: &str, buf: &String) {
    let mut folder = folder.clone();
    folder.push(filename);
    let mut fd = std::fs::File::create(folder.as_path()).unwrap();
    folder.pop();

    fd.write(buf.as_bytes()).unwrap();
}

pub fn write_json(compile_table: &CompileTable, execution_table: &ExecutionTable) {
    let itable = serde_json::to_string(&compile_table.itable).unwrap();
    let imtable = serde_json::to_string(&compile_table.imtable).unwrap();
    let etable = serde_json::to_string(&execution_table.etable).unwrap();
    let mtable = serde_json::to_string(&execution_table.mtable).unwrap();
    let jtable = serde_json::to_string(&execution_table.jtable).unwrap();

    let dir = env::current_dir().unwrap();
    write_file(&dir, "itable.json", &itable);
    write_file(&dir, "imtable.json", &imtable);
    write_file(&dir, "etable.json", &etable);
    write_file(&dir, "mtable.json", &mtable);
    write_file(&dir, "jtable.json", &jtable);
}
