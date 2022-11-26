use halo2_proofs::{
    arithmetic::FieldExt,
    plonk::{ConstraintSystem, Expression, VirtualCells},
};

use crate::circuits::etable_compact::op_configure::{
    ConstraintBuilder, EventTableCellAllocator, EventTableOpcodeConfig,
};

pub mod sha256_helper;
pub mod require_helper;
pub mod wasm_input_helper;

pub trait ForeignCallInfo {
    fn call_id(&self) -> usize;
}

pub trait EventTableForeignCallConfigBuilder<F: FieldExt> {
    fn configure(
        common: &mut EventTableCellAllocator<F>,
        constraint_builder: &mut ConstraintBuilder<F>,
        info: &impl ForeignCallInfo,
    ) -> Box<dyn EventTableOpcodeConfig<F>>;
}

pub trait ForeignTableConfig<F: FieldExt> {
    fn configure_in_table(
        &self,
        meta: &mut ConstraintSystem<F>,
        key: &'static str,
        expr: &dyn Fn(&mut VirtualCells<'_, F>) -> Expression<F>,
    );
}
