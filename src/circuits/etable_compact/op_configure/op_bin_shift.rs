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
use specs::itable::{ShiftOp, OPCODE_ARG1_SHIFT};
use specs::mtable::VarType;
use specs::step::StepInfo;
use specs::{
    etable::EventTableEntry,
    itable::{OpcodeClass, OPCODE_ARG0_SHIFT, OPCODE_CLASS_SHIFT},
};

pub struct BinShiftConfig {
    lhs: U64Cell,
    rhs: U64Cell,
    res: U64Cell,
    vtype: CommonRangeCell,
    is_shl: BitCell,
    is_shr_u: BitCell,
    lookup_stack_read_lhs: MTableLookupCell,
    lookup_stack_read_rhs: MTableLookupCell,
    lookup_stack_write: MTableLookupCell,
}

pub struct BinShiftConfigBuilder {}

impl<F: FieldExt> EventTableOpcodeConfigBuilder<F> for BinShiftConfigBuilder {
    fn configure(
        common: &mut EventTableCellAllocator<F>,
        constraint_builder: &mut ConstraintBuilder<F>,
    ) -> Box<dyn EventTableOpcodeConfig<F>> {
        let lhs = common.alloc_u64();
        let rhs = common.alloc_u64();
        let res = common.alloc_u64();

        let vtype = common.alloc_common_range_value();

        let is_shl = common.alloc_bit_value();
        let is_shr_u = common.alloc_bit_value();

        let lookup_stack_read_lhs = common.alloc_mtable_lookup();
        let lookup_stack_read_rhs = common.alloc_mtable_lookup();
        let lookup_stack_write = common.alloc_mtable_lookup();

        Box::new(BinShiftConfig {
            lhs,
            rhs,
            res,
            vtype,
            is_shl,
            is_shr_u,
            lookup_stack_read_lhs,
            lookup_stack_read_rhs,
            lookup_stack_write,
        })
    }
}

impl<F: FieldExt> EventTableOpcodeConfig<F> for BinShiftConfig {
    fn opcode(&self, meta: &mut VirtualCells<'_, F>) -> Expression<F> {
        constant!(bn_to_field(
            &(BigUint::from(OpcodeClass::BinShift as u64) << OPCODE_CLASS_SHIFT)
        )) + self.is_shl.expr(meta)
            * constant!(bn_to_field(
                &(BigUint::from(ShiftOp::Shl as u64) << OPCODE_ARG0_SHIFT)
            ))
            + self.is_shr_u.expr(meta)
                * constant!(bn_to_field(
                    &(BigUint::from(ShiftOp::UnsignedShr as u64) << OPCODE_ARG0_SHIFT)
                ))
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
            StepInfo::I32BinShiftOp {
                class,
                left,
                right,
                value,
            } => {
                let vtype = VarType::I32;
                let left = left as u32 as u64;
                let right = right as u32 as u64;
                let value = value as u32 as u64;

                (class, vtype, left, right, value)
            }

            _ => unreachable!(),
        };

        self.vtype.assign(ctx, vtype as u16)?;
        self.lhs.assign(ctx, left)?;
        self.rhs.assign(ctx, right)?;
        self.res.assign(ctx, value)?;

        match class {
            specs::itable::ShiftOp::Shl => {
                self.is_shl.assign(ctx, true)?;
            }
            specs::itable::ShiftOp::UnsignedShr => {
                self.is_shr_u.assign(ctx, true)?;
            }
        }

        self.lookup_stack_read_lhs.assign(
            ctx,
            &MemoryTableLookupEncode::encode_stack_read(
                BigUint::from(step_info.current.eid),
                BigUint::from(1 as u64),
                BigUint::from(step_info.current.sp + 1),
                BigUint::from(vtype as u64),
                BigUint::from(right),
            ),
        )?;

        self.lookup_stack_read_rhs.assign(
            ctx,
            &MemoryTableLookupEncode::encode_stack_read(
                BigUint::from(step_info.current.eid),
                BigUint::from(2 as u64),
                BigUint::from(step_info.current.sp + 2),
                BigUint::from(vtype as u64),
                BigUint::from(left),
            ),
        )?;

        self.lookup_stack_write.assign(
            ctx,
            &MemoryTableLookupEncode::encode_stack_write(
                BigUint::from(step_info.current.eid),
                BigUint::from(3 as u64),
                BigUint::from(step_info.current.sp + 2),
                BigUint::from(vtype as u64),
                BigUint::from(value),
            ),
        )?;

        Ok(())
    }

    fn opcode_class(&self) -> OpcodeClass {
        OpcodeClass::BinShift
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
            MLookupItem::First => Some(MemoryTableLookupEncode::encode_stack_read(
                common_config.eid(meta),
                constant_from!(1),
                common_config.sp(meta) + constant_from!(1),
                self.vtype.expr(meta),
                self.rhs.expr(meta),
            )),
            MLookupItem::Second => Some(MemoryTableLookupEncode::encode_stack_read(
                common_config.eid(meta),
                constant_from!(2),
                common_config.sp(meta) + constant_from!(2),
                self.vtype.expr(meta),
                self.lhs.expr(meta),
            )),
            MLookupItem::Third => Some(MemoryTableLookupEncode::encode_stack_write(
                common_config.eid(meta),
                constant_from!(3),
                common_config.sp(meta) + constant_from!(2),
                self.vtype.expr(meta),
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
    fn test_i32_shr_u_1_ok() {
        let textual_repr = r#"
                (module
                    (func (export "test")
                      (i32.const 12)
                      (i32.const 3)
                      (i32.shr_u)
                      (drop)
                    )
                   )
                "#;

        test_circuit_noexternal(textual_repr).unwrap()
    }

    #[test]
    fn test_i32_shr_u_2_ok() {
        let textual_repr = r#"
                (module
                    (func (export "test")
                      (i32.const 12)
                      (i32.const 35)
                      (i32.shr_u)
                      (drop)
                    )
                   )
                "#;

        test_circuit_noexternal(textual_repr).unwrap()
    }

    #[test]
    fn test_i32_shl_1_ok() {
        let textual_repr = r#"
                (module
                    (func (export "test")
                      (i32.const 12)
                      (i32.const 0)
                      (i32.shl)
                      (drop)
                    )
                   )
                "#;

        test_circuit_noexternal(textual_repr).unwrap()
    }

    #[test]
    fn test_i32_shl_2_ok() {
        let textual_repr = r#"
                (module
                    (func (export "test")
                      (i32.const 12)
                      (i32.const 1)
                      (i32.shl)
                      (drop)
                    )
                   )
                "#;

        test_circuit_noexternal(textual_repr).unwrap()
    }

    #[test]
    fn test_i32_shl_3_ok() {
        let textual_repr = r#"
                (module
                    (func (export "test")
                      (i32.const 12)
                      (i32.const 33)
                      (i32.shl)
                      (drop)
                    )
                   )
                "#;

        test_circuit_noexternal(textual_repr).unwrap()
    }

    #[test]
    fn test_i32_shl_overflow_ok() {
        let textual_repr = r#"
                (module
                    (func (export "test")
                      (i32.const 4294967295)
                      (i32.const 1)
                      (i32.shl)
                      (drop)
                    )
                   )
                "#;

        test_circuit_noexternal(textual_repr).unwrap()
    }
}
