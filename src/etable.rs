use crate::itable::Inst;
use halo2_proofs::arithmetic::FieldExt;
use halo2_proofs::plonk::Advice;
use halo2_proofs::plonk::Column;
use halo2_proofs::plonk::Fixed;
use std::marker::PhantomData;
use wasmi::tracer::etable::EEntry;
use wasmi::tracer::etable::RunInstructionTraceStep;

pub struct Event {
    eid: u64,
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
    cols: [Column<Advice>; 4],
    aux_cols: [Column<Advice>; 4],
}

impl EventTableConfig {
    pub fn new(cols: [Column<Advice>; 4], aux_cols: [Column<Advice>; 4]) -> Self {
        EventTableConfig { cols, aux_cols }
    }
}

pub struct EventTableChip<F: FieldExt> {
    config: EventTableConfig,
    _phantom: PhantomData<F>,
}

impl<F: FieldExt> EventTableChip<F> {
    pub fn new(config: EventTableConfig) -> Self {
        EventTableChip {
            config,
            _phantom: PhantomData,
        }
    }
}
