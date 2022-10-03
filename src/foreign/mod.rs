use halo2_proofs::arithmetic::FieldExt;

use crate::circuits::etable_compact::op_configure::{
    ConstraintBuilder, EventTableCellAllocator, EventTableOpcodeConfig,
};

pub mod sha256_helper;

pub trait ForeignCallInfo {
    fn call_id(&self) -> u64;
}

pub trait EventTableForeignCallConfigBuilder<F: FieldExt> {
    fn configure(
        common: &mut EventTableCellAllocator<F>,
        constraint_builder: &mut ConstraintBuilder<F>,
        info: &impl ForeignCallInfo,
    ) -> Box<dyn EventTableOpcodeConfig<F>>;
}
