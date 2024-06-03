use crate::circuits::etable::allocator::*;
use crate::circuits::etable::ConstraintBuilder;
use crate::circuits::etable::EventTableCommonConfig;
use crate::circuits::etable::EventTableOpcodeConfig;
use crate::circuits::etable::EventTableOpcodeConfigBuilder;
use crate::circuits::utils::bn_to_field;
use crate::circuits::utils::step_status::StepStatus;
use crate::circuits::utils::table_entry::EventTableEntryWithMemoryInfo;
use crate::circuits::utils::Context;
use crate::constant_from;
use crate::constant_from_bn;
use halo2_proofs::arithmetic::FieldExt;
use halo2_proofs::plonk::Error;
use halo2_proofs::plonk::Expression;
use halo2_proofs::plonk::VirtualCells;
use num_bigint::BigUint;
use specs::itable::OpcodeClass;
use specs::itable::OPCODE_CLASS_SHIFT;
use specs::step::StepInfo;

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
        _: &mut StepStatus<F>,
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
