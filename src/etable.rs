use crate::itable::Inst;
use halo2_proofs::arithmetic::FieldExt;
use halo2_proofs::plonk::Column;
use halo2_proofs::plonk::Fixed;
use std::marker::PhantomData;

pub struct Event {
    id: u64,
    sp: u64,
    last_just_eid: u64,
    inst: Inst,
}

pub struct EventTableConfig {
    cols: Vec<Column<Fixed>>,
}

pub struct EventTableChip<F: FieldExt> {
    config: EventTableConfig,
    _phantom: PhantomData<F>,
}
