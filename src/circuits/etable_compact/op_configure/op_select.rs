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
use specs::mtable::VarType;
use specs::step::StepInfo;
use specs::{
    etable::EventTableEntry,
    itable::{OpcodeClass, OPCODE_CLASS_SHIFT},
};

pub struct SelectConfig {
    // use alloc_u64_on_u8 just because there are not enough u64 cells.
    cond: U64OnU8Cell,
    cond_inv: UnlimitedCell,
    val1: U64Cell,
    val2: U64Cell,
    res: U64Cell,
    vtype: CommonRangeCell,
    lookup_stack_read_cond: MTableLookupCell,
    lookup_stack_read_val2: MTableLookupCell,
    lookup_stack_read_val1: MTableLookupCell,
    lookup_stack_write: MTableLookupCell,
}

pub struct SelectConfigBuilder {}

impl<F: FieldExt> EventTableOpcodeConfigBuilder<F> for SelectConfigBuilder {
    fn configure(
        common: &mut EventTableCellAllocator<F>,
        constraint_builder: &mut ConstraintBuilder<F>,
    ) -> Box<dyn EventTableOpcodeConfig<F>> {
        let cond = common.alloc_u64_on_u8();
        let cond_inv = common.alloc_unlimited_value();
        let val1 = common.alloc_u64();
        let val2 = common.alloc_u64();
        let res = common.alloc_u64();
        let vtype = common.alloc_common_range_value();

        constraint_builder.push(
            "select: cond is zero",
            Box::new(move |meta| {
                vec![
                    (constant_from!(1) - cond.expr(meta) * cond_inv.expr(meta))
                        * (res.expr(meta) - val2.expr(meta)),
                ]
            }),
        );

        constraint_builder.push(
            "select: cond is not zero",
            Box::new(move |meta| vec![cond.expr(meta) * (res.expr(meta) - val1.expr(meta))]),
        );

        let lookup_stack_read_cond = common.alloc_mtable_lookup();
        let lookup_stack_read_val2 = common.alloc_mtable_lookup();
        let lookup_stack_read_val1 = common.alloc_mtable_lookup();
        let lookup_stack_write = common.alloc_mtable_lookup();

        Box::new(SelectConfig {
            cond,
            cond_inv,
            val1,
            val2,
            res,
            vtype,
            lookup_stack_read_cond,
            lookup_stack_read_val2,
            lookup_stack_read_val1,
            lookup_stack_write,
        })
    }
}

impl<F: FieldExt> EventTableOpcodeConfig<F> for SelectConfig {
    fn opcode(&self, _meta: &mut VirtualCells<'_, F>) -> Expression<F> {
        constant!(bn_to_field(
            &(BigUint::from(OpcodeClass::Select as u64) << OPCODE_CLASS_SHIFT)
        ))
    }

    fn assign(
        &self,
        ctx: &mut Context<'_, F>,
        step_info: &StepStatus,
        entry: &EventTableEntry,
    ) -> Result<(), Error> {
        match &entry.step_info {
            StepInfo::Select {
                val1,
                val2,
                cond,
                result,
                vtype,
            } => {
                self.val1.assign(ctx, *val1)?;
                self.val2.assign(ctx, *val2)?;
                self.cond.assign(ctx, *cond)?;
                self.cond_inv
                    .assign(ctx, F::from(*cond).invert().unwrap_or(F::zero()))?;
                self.res.assign(ctx, *result)?;
                self.vtype.assign(ctx, F::from(*vtype as u64))?;

                self.lookup_stack_read_cond.assign(
                    ctx,
                    &MemoryTableLookupEncode::encode_stack_read(
                        BigUint::from(step_info.current.eid),
                        BigUint::from(1 as u64),
                        BigUint::from(step_info.current.sp + 1),
                        BigUint::from(VarType::I32 as u64),
                        BigUint::from(*cond),
                    ),
                )?;

                self.lookup_stack_read_val2.assign(
                    ctx,
                    &MemoryTableLookupEncode::encode_stack_read(
                        BigUint::from(step_info.current.eid),
                        BigUint::from(2 as u64),
                        BigUint::from(step_info.current.sp + 2),
                        BigUint::from(*vtype as u64),
                        BigUint::from(*val2),
                    ),
                )?;

                self.lookup_stack_read_val1.assign(
                    ctx,
                    &MemoryTableLookupEncode::encode_stack_read(
                        BigUint::from(step_info.current.eid),
                        BigUint::from(3 as u64),
                        BigUint::from(step_info.current.sp + 3),
                        BigUint::from(*vtype as u64),
                        BigUint::from(*val1),
                    ),
                )?;

                self.lookup_stack_write.assign(
                    ctx,
                    &MemoryTableLookupEncode::encode_stack_write(
                        BigUint::from(step_info.current.eid),
                        BigUint::from(4 as u64),
                        BigUint::from(step_info.current.sp + 3),
                        BigUint::from(*vtype as u64),
                        BigUint::from(*result),
                    ),
                )?;
            }
            _ => unreachable!(),
        }

        Ok(())
    }

    fn opcode_class(&self) -> OpcodeClass {
        OpcodeClass::Select
    }

    fn mops(&self, _meta: &mut VirtualCells<'_, F>) -> Option<Expression<F>> {
        Some(constant_from!(4))
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
                constant_from!(VarType::I32),
                self.cond.expr(meta),
            )),

            MLookupItem::Second => Some(MemoryTableLookupEncode::encode_stack_read(
                common_config.eid(meta),
                constant_from!(2),
                common_config.sp(meta) + constant_from!(2),
                self.vtype.expr(meta),
                self.val2.expr(meta),
            )),
            MLookupItem::Third => Some(MemoryTableLookupEncode::encode_stack_read(
                common_config.eid(meta),
                constant_from!(3),
                common_config.sp(meta) + constant_from!(3),
                self.vtype.expr(meta),
                self.val1.expr(meta),
            )),
            MLookupItem::Fourth => Some(MemoryTableLookupEncode::encode_stack_write(
                common_config.eid(meta),
                constant_from!(4),
                common_config.sp(meta) + constant_from!(3),
                self.vtype.expr(meta),
                self.res.expr(meta),
            )),
            _ => None,
        }
    }

    fn sp_diff(&self, _meta: &mut VirtualCells<'_, F>) -> Option<Expression<F>> {
        Some(constant_from!(2))
    }
}

#[cfg(test)]
mod tests {
    use crate::test::test_circuit_noexternal;

    #[test]
    fn test_select_i32() {
        let textual_repr = r#"
        (module
            (func (export "test")
              (i32.const 1)
              (i32.const 2)
              (i32.const 0)
              select
              drop
            )
           )
        "#;

        test_circuit_noexternal(textual_repr).unwrap();
    }

    #[test]
    fn test_select_i64() {
        let textual_repr = r#"
        (module
            (func (export "test")
              (i64.const 1)
              (i64.const 2)
              (i32.const 0)
              select
              drop
            )
           )
        "#;

        test_circuit_noexternal(textual_repr).unwrap();
    }
}
