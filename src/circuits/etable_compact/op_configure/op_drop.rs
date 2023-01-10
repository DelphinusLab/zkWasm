use super::*;
use crate::{
    circuits::utils::{bn_to_field, Context},
    constant,
};
use halo2_proofs::{
    arithmetic::FieldExt,
    plonk::{Error, Expression, VirtualCells},
};
use specs::{etable::EventTableEntry, itable::OPCODE_CLASS_SHIFT};

pub struct DropConfig {}

pub struct DropConfigBuilder {}

impl<F: FieldExt> EventTableOpcodeConfigBuilder<F> for DropConfigBuilder {
    fn configure(
        _common: &mut EventTableCellAllocator<F>,
        _constraint_builder: &mut ConstraintBuilder<F>,
    ) -> Box<dyn EventTableOpcodeConfig<F>> {
        Box::new(DropConfig {})
    }
}

impl<F: FieldExt> EventTableOpcodeConfig<F> for DropConfig {
    fn opcode(&self, _meta: &mut VirtualCells<'_, F>) -> Expression<F> {
        constant!(bn_to_field(
            &(BigUint::from(OpcodeClass::Drop as u64) << OPCODE_CLASS_SHIFT)
        ))
    }

    fn assign(
        &self,
        _ctx: &mut Context<'_, F>,
        _step: &StepStatus,
        entry: &EventTableEntry,
    ) -> Result<(), Error> {
        match &entry.step_info {
            specs::step::StepInfo::Drop => Ok(()),
            _ => unreachable!(),
        }
    }

    fn sp_diff(&self, _meta: &mut VirtualCells<'_, F>) -> Option<Expression<F>> {
        Some(constant!(F::one()))
    }
}
