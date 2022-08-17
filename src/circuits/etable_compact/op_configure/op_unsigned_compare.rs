use super::*;
use crate::{circuits::utils::Context, constant};
use halo2_proofs::{
    arithmetic::FieldExt,
    plonk::{Error, Expression, VirtualCells},
};
use specs::{
    etable::EventTableEntry,
    itable::{OpcodeClass, OPCODE_ARG0_SHIFT, OPCODE_CLASS_SHIFT},
};

pub struct CompareConfig {
    // vtype
    /*
    is_one_byte: BitCell,
    is_two_bytes: BitCell,
    is_four_bytes: BitCell,
    is_eight_bytes: BitCell,
    is_signed: BitCell,
    */
    vtype: CommonRangeCell,

    diff_inv: UnlimitedCell,
    res_is_eq: BitCell,
    res_is_lt: BitCell,
    res_is_gt: BitCell,

    left: U64Cell,
    right: U64Cell,
    diff: U64Cell,
    res: U64Cell,

    op_is_eq: BitCell,
    op_is_ne: BitCell,
    op_is_lt: BitCell,
    op_is_gt: BitCell,
    op_is_le: BitCell,
    op_is_ge: BitCell,

    lookup_stack_read_left: MTableLookupCell,
    lookup_stack_read_right: MTableLookupCell,
    lookup_stack_write_res: MTableLookupCell,
}

pub struct CompareConfigBuilder {}

impl<F: FieldExt> EventTableOpcodeConfigBuilder<F> for CompareConfigBuilder {
    fn configure(
        common: &mut EventTableCellAllocator<F>,
        constraint_builder: &mut ConstraintBuilder<F>,
    ) -> Box<dyn EventTableOpcodeConfig<F>> {
        let vtype = common.alloc_common_range_value();
        let diff_inv = common.alloc_unlimited_value();
        let res_is_eq = common.alloc_bit_value();
        let res_is_lt = common.alloc_bit_value();
        let res_is_gt = common.alloc_bit_value();
        let left = common.alloc_u64();
        let right = common.alloc_u64();
        let diff = common.alloc_u64();
        let res = common.alloc_u64();

        let op_is_eq = common.alloc_bit_value();
        let op_is_ne = common.alloc_bit_value();
        let op_is_lt = common.alloc_bit_value();
        let op_is_gt = common.alloc_bit_value();
        let op_is_le = common.alloc_bit_value();
        let op_is_ge = common.alloc_bit_value();

        let lookup_stack_read_left = common.alloc_mtable_lookup();
        let lookup_stack_read_right = common.alloc_mtable_lookup();
        let lookup_stack_write_res = common.alloc_mtable_lookup();

        constraint_builder.push(
            "compare diff",
            Box::new(move |meta| {
                vec![
                    (left.expr(meta) + res_is_lt.expr(meta) * diff.expr(meta)
                        - res_is_gt.expr(meta) * diff.expr(meta)
                        - right.expr(meta)),
                    (res_is_gt.expr(meta) + res_is_lt.expr(meta) + res_is_eq.expr(meta)
                        - constant_from!(1)),
                    (diff.expr(meta) * res_is_eq.expr(meta)),
                    (diff.expr(meta) * diff_inv.expr(meta) + res_is_eq.expr(meta)
                        - constant_from!(1)),
                ]
            }),
        );

        constraint_builder.push(
            "compare op",
            Box::new(move |meta| {
                vec![
                    (op_is_eq.expr(meta)
                        + op_is_ne.expr(meta) * op_is_lt.expr(meta)
                        + op_is_gt.expr(meta)
                        + op_is_le.expr(meta) * op_is_ge.expr(meta)
                        - constant_from!(1)),
                ]
            }),
        );

        constraint_builder.push(
            "compare op res",
            Box::new(move |meta| {
                vec![
                    op_is_eq.expr(meta) * (res.expr(meta) - res_is_eq.expr(meta)),
                    op_is_ne.expr(meta)
                        * (res.expr(meta) - constant_from!(1) + res_is_eq.expr(meta)),
                    op_is_lt.expr(meta) * (res.expr(meta) - res_is_lt.expr(meta)),
                    op_is_le.expr(meta)
                        * (res.expr(meta) - res_is_lt.expr(meta) - res_is_eq.expr(meta)),
                    op_is_gt.expr(meta) * (res.expr(meta) - res_is_gt.expr(meta)),
                    op_is_ge.expr(meta)
                        * (res.expr(meta) - res_is_gt.expr(meta) - res_is_eq.expr(meta)),
                ]
            }),
        );

        Box::new(CompareConfig {
            vtype,
            diff_inv,
            res_is_eq,
            res_is_lt,
            res_is_gt,
            left,
            right,
            diff,
            lookup_stack_read_left,
            lookup_stack_read_right,
            lookup_stack_write_res,
            res,
            op_is_eq,
            op_is_ne,
            op_is_lt,
            op_is_gt,
            op_is_le,
            op_is_ge,
        })
    }
}

impl<F: FieldExt> EventTableOpcodeConfig<F> for CompareConfig {
    fn opcode(&self, meta: &mut VirtualCells<'_, F>) -> Expression<F> {
        /*
        constant!(bn_to_field(
            &(BigUint::from(OpcodeClass::Const as u64) << OPCODE_CLASS_SHIFT)
        )) + self.vtype.expr(meta)
            * constant!(bn_to_field(&(BigUint::from(1u64) << OPCODE_ARG0_SHIFT)))
            + self.value.expr(meta)
        */
        todo!()
    }

    fn assign(
        &self,
        ctx: &mut Context<'_, F>,
        step_info: &StepStatus,
        entry: &EventTableEntry,
    ) -> Result<(), Error> {
        /*
        match &entry.step_info {
            specs::step::StepInfo::I32Const { value } => {
                self.value.assign(ctx, *value as u32 as u64)?;
                self.vtype.assign(ctx, VarType::I32 as u16)?;

                self.lookup_stack_write.assign(
                    ctx,
                    &MemoryTableConfig::<F>::encode_stack_write(
                        BigUint::from(step_info.current.eid),
                        BigUint::from(1 as u64),
                        BigUint::from(step_info.current.sp),
                        BigUint::from(VarType::I32 as u16),
                        BigUint::from(*value as u32 as u64),
                    ),
                )?;

                Ok(())
            }
            _ => unreachable!(),
        }
        */
        todo!()
    }

    fn opcode_class(&self) -> OpcodeClass {
        OpcodeClass::Const
    }

    fn sp_diff(&self, _meta: &mut VirtualCells<'_, F>) -> Option<Expression<F>> {
        Some(constant!(-F::one()))
    }

    fn mops(&self, _meta: &mut VirtualCells<'_, F>) -> Option<Expression<F>> {
        Some(constant_from!(1))
    }

    fn mtable_lookup(
        &self,
        meta: &mut VirtualCells<'_, F>,
        item: MLookupItem,
        common_config: &EventTableCommonConfig<F>,
    ) -> Option<Expression<F>> {
        todo!()
        /*
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
         */
    }
}

#[cfg(test)]
mod tests {
    use crate::test::test_circuit_builder::test_circuit_noexternal;

    #[test]
    fn test_op_const_ok() {
        let textual_repr = r#"
                (module
                    (func (export "test")
                      (i32.const 0)
                      (drop)
                    )
                   )
                "#;

        test_circuit_noexternal(textual_repr).unwrap();
    }
}
