use specs::configure_table::ConfigureTable;
use specs::itable::InstructionTable;

#[derive(Clone)]
pub struct Status<'a> {
    pub eid: u32,
    pub fid: u32,
    pub iid: u32,
    pub sp: u32,
    pub last_jump_eid: u32,
    pub allocated_memory_pages: u32,
    pub itable: &'a InstructionTable,
}

pub struct StepStatus<'a, 'b> {
    pub current: &'a Status<'b>,
    pub next: &'a Status<'b>,
    pub current_external_host_call_index: u32,
    pub host_public_inputs: u32,
    pub context_in_index: u32,
    pub context_out_index: u32,
    pub configure_table: ConfigureTable,
}
