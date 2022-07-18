use std::{env, io::Write, path::PathBuf};

use mtable::MTable;
use serde::Serialize;

use self::{
    etable::EventTableEntry, imtable::InitMemoryTableEntry, itable::InstructionTableEntry,
    jtable::JumpTableEntry,
};

pub mod etable;
pub mod imtable;
pub mod itable;
pub mod jtable;
pub mod mtable;
pub mod step;
pub mod types;

#[derive(Default, Serialize)]
pub struct CompileTable {
    pub itable: Vec<InstructionTableEntry>,
    pub imtable: Vec<InitMemoryTableEntry>,
}

impl CompileTable {
    pub fn to_string(&self) -> String {
        serde_json::to_string(self).unwrap()
    }
}

#[derive(Default, Serialize)]
pub struct ExecutionTable {
    pub etable: Vec<EventTableEntry>,
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
    write_file(&dir, "itable.data", &itable);
    write_file(&dir, "imtable.data", &imtable);
    write_file(&dir, "etable.data", &etable);
    write_file(&dir, "mtable.data", &mtable);
    write_file(&dir, "jtable.data", &jtable);
}
