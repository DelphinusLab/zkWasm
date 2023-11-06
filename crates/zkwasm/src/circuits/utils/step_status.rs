#[derive(Clone)]
pub struct Status {
    pub eid: u32,
    pub fid: u32,
    pub iid: u32,
    pub sp: u32,
    pub last_jump_eid: u32,
    pub allocated_memory_pages: u32,
}

pub struct StepStatus<'a> {
    pub current: &'a Status,
    pub next: &'a Status,
    pub current_external_host_call_index: u32,
    pub host_public_inputs: u32,
    pub context_in_index: u32,
    pub context_out_index: u32,
    pub maximal_memory_pages: u32,
}
