use crate::circuits::cell::AllocatedUnlimitedCell;
use crate::circuits::config::zkwasm_k;
use crate::circuits::etable::allocator::EventTableCellAllocator;
use crate::circuits::etable::constraint_builder::ConstraintBuilder;
use crate::circuits::etable::EventTableCommonConfig;
use crate::circuits::etable::EventTableOpcodeConfig;
use halo2_proofs::arithmetic::FieldExt;
use halo2_proofs::plonk::ConstraintSystem;
use halo2_proofs::plonk::Expression;
use halo2_proofs::plonk::VirtualCells;

pub mod context;
pub mod log_helper;
pub mod require_helper;
pub mod wasm_input_helper;

pub fn foreign_table_enable_lines() -> usize {
    1 << (zkwasm_k() - 1)
}

pub trait ForeignTableConfig<F: FieldExt> {
    fn configure_in_table(
        &self,
        meta: &mut ConstraintSystem<F>,
        name: &'static str,
        expr: &dyn Fn(&mut VirtualCells<'_, F>) -> Vec<Expression<F>>,
    );
}

pub(crate) trait EventTableForeignCallConfigBuilder<F: FieldExt> {
    fn configure(
        self,
        common_config: &EventTableCommonConfig<F>,
        allocator: &mut EventTableCellAllocator<F>,
        constraint_builder: &mut ConstraintBuilder<F>,
        lookup_cells: &mut (impl Iterator<Item = AllocatedUnlimitedCell<F>> + Clone),
    ) -> Box<dyn EventTableOpcodeConfig<F>>;
}

pub(crate) trait InternalHostPluginBuilder {
    fn new(index: usize) -> Self;
}
