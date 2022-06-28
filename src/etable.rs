use crate::itable::Inst;
use halo2_proofs::arithmetic::FieldExt;
use halo2_proofs::plonk::Column;
use halo2_proofs::plonk::Fixed;
use std::marker::PhantomData;
use wasmi::tracer::etable::EEntry;
use wasmi::tracer::etable::RunInstructionTraceStep;

pub struct Event {
    id: u64,
    sp: u64,
    last_just_eid: u64,
    inst: Inst,
    step_info: RunInstructionTraceStep,
}

impl From<EEntry> for Event {
    fn from(e_entry: EEntry) -> Self {
        Event {
            id: e_entry.id,
            sp: e_entry.sp,
            last_just_eid: 0,
            inst: Inst::from(e_entry.inst),
            step_info: e_entry.step,
        }
    }
}

pub struct EventTableConfig {
    cols: Vec<Column<Fixed>>,
}

pub struct EventTableChip<F: FieldExt> {
    config: EventTableConfig,
    _phantom: PhantomData<F>,
}
