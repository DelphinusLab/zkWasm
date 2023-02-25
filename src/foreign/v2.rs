use halo2_proofs::arithmetic::FieldExt;

use crate::circuits::etable_v2::{
    allocator::EventTableCellAllocator, constraint_builder::ConstraintBuilder,
    EventTableCommonConfig, EventTableOpcodeConfig,
};

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
