use super::itable::InstructionTableEntry;
use wasmi::tracer::etable::{EEntry, RunInstructionTraceStep};

#[derive(Clone)]
pub struct EventTableEntry {
    pub(crate) eid: u64,
    pub(crate) sp: u64,
    pub(crate) last_jump_eid: u64,
    pub(crate) inst: InstructionTableEntry,
    pub(crate) step_info: RunInstructionTraceStep,
}

impl From<&EEntry> for EventTableEntry {
    fn from(e_entry: &EEntry) -> Self {
        EventTableEntry {
            eid: e_entry.id,
            sp: e_entry.sp,
            // FIXME: fill with correct value
            last_jump_eid: 0,
            inst: InstructionTableEntry::from(&e_entry.inst),
            step_info: e_entry.step.clone(),
        }
    }
}
