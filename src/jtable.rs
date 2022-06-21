use crate::itable::Inst;
use halo2_proofs::arithmetic::FieldExt;
use halo2_proofs::plonk::Column;
use halo2_proofs::plonk::Fixed;
use std::marker::PhantomData;

pub struct Jump {
    eid: u64,
    last_jump_eid: u64,
    inst: Inst,
}

pub struct EventTableConfig {
    cols: [Column<Fixed>; 3],
}

pub struct EventTableChip<F: FieldExt> {
    config: EventTableConfig,
    _phantom: PhantomData<F>,
}
