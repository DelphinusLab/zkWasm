use super::*;
use crate::{
    circuits::{mtable_compact::expression::MtableLookupEntryEncode, utils::Context},
    constant,
};
use halo2_proofs::{
    arithmetic::FieldExt,
    plonk::{ConstraintSystem, Error, Expression, VirtualCells},
};
use specs::{etable::EventTableEntry, itable::OpcodeClass, mtable::VarType};

pub struct ConstConfig {
    vtype: CommonRangeCell,
    value: U64Cell,
}

pub struct ConstConfigBuilder {}

impl<F: FieldExt> EventTableOpcodeConfigBuilder<F> for ConstConfigBuilder {
    fn configure(
        _meta: &mut ConstraintSystem<F>,
        common: &mut EventTableCellAllocator<F>,
        _enable: impl Fn(&mut VirtualCells<'_, F>) -> Expression<F>,
    ) -> Box<dyn EventTableOpcodeConfig<F>> {
        let vtype = common.alloc_common_range_value();
        let value = common.alloc_u64();

        Box::new(ConstConfig { vtype, value })
    }
}

impl<F: FieldExt> EventTableOpcodeConfig<F> for ConstConfig {
    fn opcode(&self, meta: &mut VirtualCells<'_, F>) -> Expression<F> {
        todo!()
    }

    fn assign(&self, ctx: &mut Context<'_, F>, entry: &EventTableEntry) -> Result<(), Error> {
        match &entry.step_info {
            specs::step::StepInfo::I32Const { value } => {
                self.value.assign(ctx, *value as u64)?;
                self.vtype.assign(ctx, VarType::I32 as u16)?;

                Ok(())
            }
            _ => unreachable!(),
        }
    }

    fn opcode_class(&self) -> OpcodeClass {
        OpcodeClass::Const
    }

    fn sp_diff(&self, _meta: &mut VirtualCells<'_, F>) -> Option<Expression<F>> {
        Some(constant!(-F::one()))
    }

    fn mtable_lookup(
        &self,
        meta: &mut VirtualCells<'_, F>,
        item: MLookupItem,
        common_config: &EventTableCommonConfig<F>,
    ) -> Option<Expression<F>> {
        match item {
            MLookupItem::First => Some(MemoryTableConfig::encode_stack_write(
                common_config.eid(meta),
                constant_from!(1),
                common_config.sp(meta),
                self.vtype.expr(meta),
                self.value.expr(meta),
            )),
            MLookupItem::Second => None,
            MLookupItem::Third => None,
            MLookupItem::Fourth => None,
        }
    }
}
