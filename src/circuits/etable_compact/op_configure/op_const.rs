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
    stack_write_lookup: MTableLookupCell,
}

pub struct ConstConfigBuilder {}

impl<F: FieldExt> EventTableOpcodeConfigBuilder<F> for ConstConfigBuilder {
    fn configure(
        meta: &mut ConstraintSystem<F>,
        common: &mut EventTableCellAllocator<F>,
        enable: impl Fn(&mut VirtualCells<'_, F>) -> Expression<F>,
    ) -> Box<dyn EventTableOpcodeConfig<F>> {
        let vtype = common.alloc_common_range_value();
        let value = common.alloc_u64();
        let stack_write_lookup = common.alloc_mtable_lookup();

        Box::new(ConstConfig {
            vtype,
            value,
            stack_write_lookup,
        })
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
        mtable: &MemoryTableConfig<F>,
        item: MLookupItem,
    ) -> Option<Expression<F>> {
        match item {
            MLookupItem::First => Some(mtable.encode_stack_write(
                todo!(),
                todo!(),
                todo!(),
                todo!(),
                curr!(meta, self.vtype.col),       // FIXME: curr or ?
                curr!(meta, self.value.value_col), // FIXME: curr or?
            )),
            MLookupItem::Second => None,
            MLookupItem::Third => None,
            MLookupItem::Fourth => None,
        }
    }
}
