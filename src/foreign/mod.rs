use crate::circuits::etable::{
    allocator::EventTableCellAllocator, constraint_builder::ConstraintBuilder,
    EventTableCommonConfig, EventTableOpcodeConfig,
};
use halo2_proofs::arithmetic::FieldExt;
use halo2_proofs::{
    plonk::{ConstraintSystem, Expression, VirtualCells},
};

pub mod keccak_helper;
pub mod log_helper;
pub mod require_helper;
pub mod wasm_input_helper;

pub trait ForeignTableConfig<F: FieldExt> {
    fn configure_in_table(
        &self,
        meta: &mut ConstraintSystem<F>,
        key: &'static str,
        expr: &dyn Fn(&mut VirtualCells<'_, F>) -> Expression<F>,
    );
}

pub(crate) trait EventTableForeignCallConfigBuilder<F: FieldExt> {
    fn configure(
        self,
        common_config: &EventTableCommonConfig<F>,
        allocator: &mut EventTableCellAllocator<F>,
        constraint_builder: &mut ConstraintBuilder<F>,
    ) -> Box<dyn EventTableOpcodeConfig<F>>;
}

pub(crate) trait InternalHostPluginBuilder {
    fn new(index: usize) -> Self;
}
