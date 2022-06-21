use halo2_proofs::{plonk::{Column, Advice, ConstraintSystem}, arithmetic::FieldExt};

enum MemTrace {
    ModuleTrace(u64, u64, u64),
    LocalTrace(u64, u64, u64)
}

const MEM_TRACE_COLUMNS: usize = 4usize;

#[derive(Clone, Debug)]
struct MAddressConfig {
    columns: [Column<Advice>; MEM_TRACE_COLUMNS]
}

impl MAddressConfig {
    pub fn new(columns: [Column<Advice>; MEM_TRACE_COLUMNS]) -> MAddressConfig {
        MAddressConfig { columns }
    }

    pub fn configure<F: FieldExt>(
        &mut self,
        meta: &mut ConstraintSystem<F>,
    ) {
    }
}
