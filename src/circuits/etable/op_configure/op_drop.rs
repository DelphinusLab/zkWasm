use crate::{
    circuits::{
        etable::{
            allocator::*, ConstraintBuilder, EventTableCommonConfig, EventTableOpcodeConfig,
            EventTableOpcodeConfigBuilder,
        },
        utils::{
            bn_to_field, step_status::StepStatus, table_entry::EventTableEntryWithMemoryInfo,
            Context,
        },
    },
    constant_from, constant_from_bn,
};
use halo2_proofs::{
    arithmetic::FieldExt,
    plonk::{Error, Expression, VirtualCells},
};
use num_bigint::BigUint;
use specs::{
    itable::{OpcodeClass, OPCODE_CLASS_SHIFT},
    step::StepInfo,
};

pub struct DropConfig;

pub struct DropConfigBuilder;

impl<F: FieldExt> EventTableOpcodeConfigBuilder<F> for DropConfigBuilder {
    fn configure(
        _: &EventTableCommonConfig<F>,
        _: &mut EventTableCellAllocator<F>,
        _: &mut ConstraintBuilder<F>,
    ) -> Box<dyn EventTableOpcodeConfig<F>> {
        Box::new(DropConfig)
    }
}

impl<F: FieldExt> EventTableOpcodeConfig<F> for DropConfig {
    fn opcode(&self, _: &mut VirtualCells<'_, F>) -> Expression<F> {
        constant_from_bn!(&(BigUint::from(OpcodeClass::Drop as u64) << OPCODE_CLASS_SHIFT))
    }

    fn assign(
        &self,
        _: &mut Context<'_, F>,
        _: &StepStatus,
        entry: &EventTableEntryWithMemoryInfo,
    ) -> Result<(), Error> {
        match &entry.eentry.step_info {
            StepInfo::Drop => Ok(()),
            _ => unreachable!(),
        }
    }

    fn sp_diff(&self, _: &mut VirtualCells<'_, F>) -> Option<Expression<F>> {
        Some(constant_from!(1))
    }
}
