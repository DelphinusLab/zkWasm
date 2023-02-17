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
use specs::{encode::opcode::encode_global_get, step::StepInfo};

pub struct GlobalGetConfig {
    idx: CommonRangeCell,
    vtype: CommonRangeCell,
    value: U64Cell,
    lookup_global_read: MTableLookupCell,
    lookup_stack_write: MTableLookupCell,
}

pub struct GlobalGetConfigBuilder {}

impl<F: FieldExt> EventTableOpcodeConfigBuilder<F> for GlobalGetConfigBuilder {
    fn configure(
        common: &mut EventTableCellAllocator<F>,
        _constraint_builder: &mut ConstraintBuilder<F>,
    ) -> Box<dyn EventTableOpcodeConfig<F>> {
        let idx = common.alloc_common_range_value();

        let vtype = common.alloc_common_range_value();
        let value = common.alloc_u64();

        let lookup_global_read = common.alloc_mtable_lookup();
        let lookup_stack_write = common.alloc_mtable_lookup();

        Box::new(GlobalGetConfig {
            idx,
            vtype,
            value,
            lookup_global_read,
            lookup_stack_write,
        })
    }
}

impl<F: FieldExt> EventTableOpcodeConfig<F> for GlobalGetConfig {
    fn opcode(&self, meta: &mut VirtualCells<'_, F>) -> Expression<F> {
        encode_global_get(self.idx.expr(meta))
    }

    fn assign(
        &self,
        ctx: &mut Context<'_, F>,
        step_info: &StepStatus,
        entry: &EventTableEntry,
    ) -> Result<(), Error> {
        match &entry.step_info {
            StepInfo::GetGlobal {
                idx, vtype, value, ..
            } => {
                self.idx.assign(ctx, F::from(*idx as u64))?;
                self.vtype.assign(ctx, F::from(*vtype as u64))?;
                self.value.assign(ctx, *value)?;

                self.lookup_global_read.assign(
                    ctx,
                    &MemoryTableLookupEncode::encode_global_get(
                        BigUint::from(step_info.current.eid),
                        BigUint::from(1 as u64),
                        BigUint::from(*idx as u64),
                        BigUint::from(*vtype as u64),
                        BigUint::from(*value),
                    ),
                )?;

                self.lookup_stack_write.assign(
                    ctx,
                    &MemoryTableLookupEncode::encode_stack_write(
                        BigUint::from(step_info.current.eid),
                        BigUint::from(2 as u64),
                        BigUint::from(step_info.current.sp),
                        BigUint::from(*vtype as u16),
                        BigUint::from(*value),
                    ),
                )?;

                Ok(())
            }

            _ => unreachable!(),
        }
    }

    fn sp_diff(&self, _meta: &mut VirtualCells<'_, F>) -> Option<Expression<F>> {
        Some(constant!(-F::one()))
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
            MLookupItem::First => Some(MemoryTableLookupEncode::encode_global_get(
                common_config.eid(meta),
                constant_from!(1),
                self.idx.expr(meta),
                self.vtype.expr(meta),
                self.value.expr(meta),
            )),
            MLookupItem::Second => Some(MemoryTableLookupEncode::encode_stack_write(
                common_config.eid(meta),
                constant_from!(2),
                common_config.sp(meta),
                self.vtype.expr(meta),
                self.value.expr(meta),
            )),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::test::test_circuit_noexternal;

    #[test]
    fn test_global_get() {
        let textual_repr = r#"
                (module
                    (global $global_i32 i32 (i32.const 10))

                    (func (export "test")
                        (global.get $global_i32)
                        (drop)
                    )
                )
                "#;

        test_circuit_noexternal(textual_repr).unwrap()
    }
}
