use mtable::MTable;

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

#[derive(Default)]
pub struct CompileTable {
    pub itable: Vec<InstructionTableEntry>,
    pub imtable: Vec<InitMemoryTableEntry>,
}

#[derive(Default)]
pub struct ExecutionTable {
    pub etable: Vec<EventTableEntry>,
    pub mtable: MTable,
    pub jtable: Vec<JumpTableEntry>,
}
