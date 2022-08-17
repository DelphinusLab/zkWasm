use super::*;
use crate::{
    circuits::utils::{bn_to_field, Context},
    constant,
};
use halo2_proofs::{
    arithmetic::FieldExt,
    plonk::{ConstraintSystem, Error, Expression, VirtualCells},
};
use specs::itable::{RelOp, OPCODE_ARG1_SHIFT};
use specs::mtable::VarType;
use specs::step::StepInfo;
use specs::{
    etable::EventTableEntry,
    itable::{OpcodeClass, OPCODE_ARG0_SHIFT, OPCODE_CLASS_SHIFT},
};

pub struct RelConfig {
    lhs: U64Cell,
    rhs: U64Cell,
    res: BitCell,
    vtype: CommonRangeCell,
    is_le: BitCell,
    is_lt: BitCell,
    is_signed: BitCell,
    lookup_stack_read_lhs: MTableLookupCell,
    lookup_stack_read_rhs: MTableLookupCell,
    lookup_stack_write: MTableLookupCell,
}

pub struct RelConfigBuilder {}

impl<F: FieldExt> EventTableOpcodeConfigBuilder<F> for RelConfigBuilder {
    fn configure(
        common: &mut EventTableCellAllocator<F>,
        constraint_builder: &mut ConstraintBuilder<F>,
    ) -> Box<dyn EventTableOpcodeConfig<F>> {
        let lhs = common.alloc_u64();
        let rhs = common.alloc_u64();
        let res = common.alloc_bit_value();

        let is_le = common.alloc_bit_value();
        let is_lt = common.alloc_bit_value();
        let is_signed = common.alloc_bit_value();

        let vtype = common.alloc_common_range_value();

        let lookup_stack_read_lhs = common.alloc_mtable_lookup();
        let lookup_stack_read_rhs = common.alloc_mtable_lookup();
        let lookup_stack_write = common.alloc_mtable_lookup();

        constraint_builder.push(
            "opcode select one",
            Box::new(move |meta| vec![(is_le.expr(meta) + is_lt.expr(meta) - constant_from!(1))]),
        );

        Box::new(RelConfig {
            lhs,
            rhs,
            res,
            is_le,
            is_lt,
            vtype,
            is_signed,
            lookup_stack_read_lhs,
            lookup_stack_read_rhs,
            lookup_stack_write,
        })
    }
}

impl<F: FieldExt> EventTableOpcodeConfig<F> for RelConfig {
    fn opcode(&self, meta: &mut VirtualCells<'_, F>) -> Expression<F> {
        let subop_lt_u = |meta: &mut VirtualCells<F>| {
            self.is_lt.expr(meta)
                * (constant_from!(1) - self.is_signed.expr(meta))
                * constant!(bn_to_field(
                    &(BigUint::from(RelOp::UnsignedLt as u64) << OPCODE_ARG0_SHIFT)
                ))
        };
        let subop_le_u = |meta: &mut VirtualCells<F>| {
            self.is_le.expr(meta)
                * (constant_from!(1) - self.is_signed.expr(meta))
                * constant!(bn_to_field(
                    &(BigUint::from(RelOp::UnsignedLe as u64) << OPCODE_ARG0_SHIFT)
                ))
        };

        let subop = |meta: &mut VirtualCells<F>| subop_le_u(meta) + subop_lt_u(meta);

        constant!(bn_to_field(
            &(BigUint::from(OpcodeClass::Rel as u64) << OPCODE_CLASS_SHIFT)
        )) + subop(meta)
            + self.vtype.expr(meta)
                * constant!(bn_to_field(&(BigUint::from(1u64) << OPCODE_ARG1_SHIFT)))
    }

    fn assign(
        &self,
        ctx: &mut Context<'_, F>,
        step_info: &StepStatus,
        entry: &EventTableEntry,
    ) -> Result<(), Error> {
        let (class, vtype, left, right, value) = match entry.step_info {
            StepInfo::I32Comp {
                class,
                left,
                right,
                value,
            } => {
                let vtype = VarType::I32;
                let left = left as u32 as u64;
                let right = right as u32 as u64;

                (class, vtype, left, right, value)
            }

            _ => unreachable!(),
        };

        self.vtype.assign(ctx, vtype as u16)?;
        self.lhs.assign(ctx, left)?;
        self.rhs.assign(ctx, right)?;
        self.res.assign(ctx, value)?;

        match class {
            RelOp::Eq => todo!(),
            RelOp::Ne => todo!(),
            RelOp::SignedGt => todo!(),
            RelOp::UnsignedGt => todo!(),
            RelOp::SignedGe => todo!(),
            RelOp::UnsignedGe => todo!(),
            RelOp::UnsignedLt => {
                self.is_lt.assign(ctx, true)?;
            }
            RelOp::UnsignedLe => {
                self.is_le.assign(ctx, true)?;
            }
        };

        self.lookup_stack_read_lhs.assign(
            ctx,
            &MemoryTableConfig::<F>::encode_stack_read(
                BigUint::from(step_info.current.eid),
                BigUint::from(1 as u64),
                BigUint::from(step_info.current.sp + 1),
                BigUint::from(vtype as u16),
                BigUint::from(right),
            ),
        )?;

        self.lookup_stack_read_rhs.assign(
            ctx,
            &MemoryTableConfig::<F>::encode_stack_read(
                BigUint::from(step_info.current.eid),
                BigUint::from(2 as u64),
                BigUint::from(step_info.current.sp + 2),
                BigUint::from(vtype as u16),
                BigUint::from(left),
            ),
        )?;

        self.lookup_stack_write.assign(
            ctx,
            &MemoryTableConfig::<F>::encode_stack_write(
                BigUint::from(step_info.current.eid),
                BigUint::from(3 as u64),
                BigUint::from(step_info.current.sp + 2),
                BigUint::from(VarType::I32 as u64),
                BigUint::from(value as u64),
            ),
        )?;

        Ok(())
    }

    fn opcode_class(&self) -> OpcodeClass {
        OpcodeClass::Rel
    }

    fn mops(&self, _meta: &mut VirtualCells<'_, F>) -> Option<Expression<F>> {
        Some(constant_from!(3))
    }

    fn mtable_lookup(
        &self,
        meta: &mut VirtualCells<'_, F>,
        item: MLookupItem,
        common_config: &EventTableCommonConfig<F>,
    ) -> Option<Expression<F>> {
        match item {
            MLookupItem::First => Some(MemoryTableConfig::<F>::encode_stack_read(
                common_config.eid(meta),
                constant_from!(1),
                common_config.sp(meta) + constant_from!(1),
                self.vtype.expr(meta),
                self.rhs.expr(meta),
            )),
            MLookupItem::Second => Some(MemoryTableConfig::<F>::encode_stack_read(
                common_config.eid(meta),
                constant_from!(2),
                common_config.sp(meta) + constant_from!(2),
                self.vtype.expr(meta),
                self.lhs.expr(meta),
            )),
            MLookupItem::Third => Some(MemoryTableConfig::<F>::encode_stack_write(
                common_config.eid(meta),
                constant_from!(3),
                common_config.sp(meta) + constant_from!(2),
                constant_from!(VarType::I32),
                self.res.expr(meta),
            )),
            MLookupItem::Fourth => None,
        }
    }

    fn sp_diff(&self, _meta: &mut VirtualCells<'_, F>) -> Option<Expression<F>> {
        Some(constant!(F::one()))
    }
}

#[cfg(test)]
mod tests {
    use crate::test::test_circuit_builder::test_circuit_noexternal;

    #[test]
    fn test_i32_le_u_1_ok() {
        let textual_repr = r#"
                (module
                    (func (export "test")
                      (i32.const 1)
                      (i32.const 0)
                      (i32.le_u)
                      (drop)
                    )
                   )
                "#;

        test_circuit_noexternal(textual_repr).unwrap()
    }

    #[test]
    fn test_i32_le_u_2_ok() {
        let textual_repr = r#"
                (module
                    (func (export "test")
                      (i32.const 1)
                      (i32.const 1)
                      (i32.le_u)
                      (drop)
                    )
                   )
                "#;

        test_circuit_noexternal(textual_repr).unwrap()
    }

    #[test]
    fn test_i32_le_u_3_ok() {
        let textual_repr = r#"
                (module
                    (func (export "test")
                      (i32.const 0)
                      (i32.const 1)
                      (i32.le_u)
                      (drop)
                    )
                   )
                "#;

        test_circuit_noexternal(textual_repr).unwrap()
    }

    #[test]
    fn test_i32_lt_u_1_ok() {
        let textual_repr = r#"
                (module
                    (func (export "test")
                      (i32.const 1)
                      (i32.const 0)
                      (i32.lt_u)
                      (drop)
                    )
                   )
                "#;

        test_circuit_noexternal(textual_repr).unwrap()
    }

    #[test]
    fn test_i32_lt_u_2_ok() {
        let textual_repr = r#"
                (module
                    (func (export "test")
                      (i32.const 1)
                      (i32.const 1)
                      (i32.lt_u)
                      (drop)
                    )
                   )
                "#;

        test_circuit_noexternal(textual_repr).unwrap()
    }

    #[test]
    fn test_i32_lt_u_3_ok() {
        let textual_repr = r#"
                (module
                    (func (export "test")
                      (i32.const 0)
                      (i32.const 1)
                      (i32.lt_u)
                      (drop)
                    )
                   )
                "#;

        test_circuit_noexternal(textual_repr).unwrap()
    }
}
