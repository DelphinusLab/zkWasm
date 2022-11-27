use super::*;
use crate::{
    circuits::{
        mtable_compact::encode::MemoryTableLookupEncode,
        utils::{bn_to_field, Context},
    },
    constant,
};
use halo2_proofs::{
    arithmetic::FieldExt,
    plonk::{Error, Expression, VirtualCells},
};
use specs::step::StepInfo;
use specs::{
    etable::EventTableEntry,
    itable::{OpcodeClass, OPCODE_ARG0_SHIFT, OPCODE_CLASS_SHIFT},
};

pub struct LocalSetConfig {
    offset: CommonRangeCell,
    vtype: CommonRangeCell,
    value: U64Cell,
    lookup_stack_read: MTableLookupCell,
    lookup_stack_write: MTableLookupCell,
}

pub struct LocalSetConfigBuilder {}

impl<F: FieldExt> EventTableOpcodeConfigBuilder<F> for LocalSetConfigBuilder {
    fn configure(
        common: &mut EventTableCellAllocator<F>,
        _constraint_builder: &mut ConstraintBuilder<F>,
    ) -> Box<dyn EventTableOpcodeConfig<F>> {
        let offset = common.alloc_common_range_value();
        let vtype = common.alloc_common_range_value();
        let value = common.alloc_u64();
        let lookup_stack_read = common.alloc_mtable_lookup();
        let lookup_stack_write = common.alloc_mtable_lookup();

        Box::new(LocalSetConfig {
            offset,
            vtype,
            value,
            lookup_stack_read,
            lookup_stack_write,
        })
    }
}

impl<F: FieldExt> EventTableOpcodeConfig<F> for LocalSetConfig {
    fn opcode(&self, meta: &mut VirtualCells<'_, F>) -> Expression<F> {
        constant!(bn_to_field(
            &(BigUint::from(OpcodeClass::LocalSet as u64) << OPCODE_CLASS_SHIFT)
        )) + self.vtype.expr(meta)
            * constant!(bn_to_field(&(BigUint::from(1u64) << OPCODE_ARG0_SHIFT)))
            + self.offset.expr(meta)
    }

    fn assign(
        &self,
        ctx: &mut Context<'_, F>,
        step_info: &StepStatus,
        entry: &EventTableEntry,
    ) -> Result<(), Error> {
        match &entry.step_info {
            StepInfo::SetLocal {
                vtype,
                depth,
                value,
            } => {
                self.vtype.assign(ctx, F::from(*vtype as u64))?;
                self.value.assign(ctx, *value)?;
                self.offset.assign(ctx, F::from(*depth as u64))?;

                self.lookup_stack_read.assign(
                    ctx,
                    &MemoryTableLookupEncode::encode_stack_read(
                        BigUint::from(step_info.current.eid),
                        BigUint::from(1 as u64),
                        BigUint::from(step_info.current.sp + 1 as u64),
                        BigUint::from(*vtype as u16),
                        BigUint::from(*value),
                    ),
                )?;

                self.lookup_stack_write.assign(
                    ctx,
                    &MemoryTableLookupEncode::encode_stack_write(
                        BigUint::from(step_info.current.eid),
                        BigUint::from(2 as u64),
                        BigUint::from(step_info.current.sp + 1 + *depth as u64),
                        BigUint::from(*vtype as u16),
                        BigUint::from(*value),
                    ),
                )?;

                Ok(())
            }

            _ => unreachable!(),
        }
    }

    fn opcode_class(&self) -> OpcodeClass {
        OpcodeClass::LocalSet
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
            MLookupItem::Second => Some(MemoryTableLookupEncode::encode_stack_write(
                common_config.eid(meta),
                constant_from!(2),
                common_config.sp(meta) + constant_from!(1) + self.offset.expr(meta),
                self.vtype.expr(meta),
                self.value.expr(meta),
            )),
            MLookupItem::Third => None,
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::test::test_circuit_noexternal;

    #[test]
    fn test_local_set() {
        let textual_repr = r#"
                (module
                    (func (export "test")
                        (local i32 i32)
                        (i32.const 1)
                        (local.set 1)
                    )
                )
                "#;

        test_circuit_noexternal(textual_repr).unwrap()
    }
}
