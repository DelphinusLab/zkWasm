use super::*;
use crate::{
    circuits::{mtable_compact::encode::MemoryTableLookupEncode, utils::Context},
    constant,
};
use halo2_proofs::{
    arithmetic::FieldExt,
    plonk::{Error, Expression, VirtualCells},
};
use specs::etable::EventTableEntry;
use specs::{encode::opcode::encode_global_set, step::StepInfo};

pub struct GlobalSetConfig {
    idx: CommonRangeCell,
    vtype: CommonRangeCell,
    value: U64Cell,
    lookup_stack_read: MTableLookupCell,
    lookup_global_set: MTableLookupCell,
}

pub struct GlobalSetConfigBuilder {}

impl<F: FieldExt> EventTableOpcodeConfigBuilder<F> for GlobalSetConfigBuilder {
    fn configure(
        common: &mut EventTableCellAllocator<F>,
        _constraint_builder: &mut ConstraintBuilder<F>,
    ) -> Box<dyn EventTableOpcodeConfig<F>> {
        let idx = common.alloc_common_range_value();

        let vtype = common.alloc_common_range_value();
        let value = common.alloc_u64();

        let lookup_stack_read = common.alloc_mtable_lookup();
        let lookup_global_set = common.alloc_mtable_lookup();

        Box::new(GlobalSetConfig {
            idx,
            vtype,
            value,
            lookup_stack_read,
            lookup_global_set,
        })
    }
}

impl<F: FieldExt> EventTableOpcodeConfig<F> for GlobalSetConfig {
    fn opcode(&self, meta: &mut VirtualCells<'_, F>) -> Expression<F> {
        encode_global_set(self.idx.expr(meta))
    }

    fn assign(
        &self,
        ctx: &mut Context<'_, F>,
        step_info: &StepStatus,
        entry: &EventTableEntry,
    ) -> Result<(), Error> {
        match &entry.step_info {
            StepInfo::SetGlobal {
                idx, vtype, value, ..
            } => {
                self.idx.assign(ctx, F::from(*idx as u64))?;
                self.vtype.assign(ctx, F::from(*vtype as u64))?;
                self.value.assign(ctx, *value)?;

                self.lookup_stack_read.assign(
                    ctx,
                    &MemoryTableLookupEncode::encode_stack_read(
                        BigUint::from(step_info.current.eid),
                        BigUint::from(1 as u64),
                        BigUint::from(step_info.current.sp + 1),
                        BigUint::from(*vtype as u16),
                        BigUint::from(*value),
                    ),
                )?;

                self.lookup_global_set.assign(
                    ctx,
                    &MemoryTableLookupEncode::encode_global_set(
                        BigUint::from(step_info.current.eid),
                        BigUint::from(2 as u64),
                        BigUint::from(*idx as u64),
                        BigUint::from(*vtype as u64),
                        BigUint::from(*value),
                    ),
                )?;

                Ok(())
            }

            _ => unreachable!(),
        }
    }

    fn sp_diff(&self, _meta: &mut VirtualCells<'_, F>) -> Option<Expression<F>> {
        Some(constant!(F::one()))
    }

    fn mops(&self, _meta: &mut VirtualCells<'_, F>) -> Option<Expression<F>> {
        Some(constant_from!(2))
    }

    fn mtable_lookup(
        &self,
        meta: &mut VirtualCells<'_, F>,
        item: MLookupItem,
        common_config: &EventTableCommonConfig<F>,
    ) -> Option<Expression<F>> {
        match item {
            MLookupItem::First => Some(MemoryTableLookupEncode::encode_stack_read(
                common_config.eid(meta),
                constant_from!(1),
                common_config.sp(meta) + constant_from!(1),
                self.vtype.expr(meta),
                self.value.expr(meta),
            )),
            MLookupItem::Second => Some(MemoryTableLookupEncode::encode_global_set(
                common_config.eid(meta),
                constant_from!(2),
                self.idx.expr(meta),
                self.vtype.expr(meta),
                self.value.expr(meta),
            )),
            _ => None,
        }
    }
}
