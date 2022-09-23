use super::*;
use crate::{
    circuits::{
        mtable_compact::encode::MemoryTableLookupEncode,
        utils::{bn_to_field, Context},
    },
    constant,
};
use ark_std::One;
use halo2_proofs::{
    arithmetic::FieldExt,
    plonk::{Error, Expression, VirtualCells},
};
use num_traits::ToPrimitive;
use specs::itable::{BinOp, OPCODE_ARG1_SHIFT};
use specs::mtable::VarType;
use specs::step::StepInfo;
use specs::{
    etable::EventTableEntry,
    itable::{OpcodeClass, OPCODE_ARG0_SHIFT, OPCODE_CLASS_SHIFT},
};

pub struct BinConfig {
    lhs: U64Cell,
    rhs: U64Cell,
    res: U64Cell,
    overflow: U64OnU8Cell,
    r: U64OnU8Cell, // remainder for div_u and quotient for rem_u
    vtype: CommonRangeCell,
    is_add_sub: BitCell,
    is_mul_div: BitCell,
    mode: BitCell,
    is_32bits: BitCell,
    is_64bits: BitCell,
    //TODO: add constraints between vtype and is_32bits, is_64bits
    lookup_stack_read_lhs: MTableLookupCell,
    lookup_stack_read_rhs: MTableLookupCell,
    lookup_stack_write: MTableLookupCell,
}

pub struct BinConfigBuilder {}

/* Common binary operation that isomorphic to unsigned binary arith */

impl<F: FieldExt> EventTableOpcodeConfigBuilder<F> for BinConfigBuilder {
    fn configure(
        common: &mut EventTableCellAllocator<F>,
        constraint_builder: &mut ConstraintBuilder<F>,
    ) -> Box<dyn EventTableOpcodeConfig<F>> {
        let lhs = common.alloc_u64();
        let rhs = common.alloc_u64();
        let res = common.alloc_u64();
        let overflow = common.alloc_u64_on_u8();
        let r = common.alloc_u64_on_u8();

        let vtype = common.alloc_common_range_value();

        let is_add_sub = common.alloc_bit_value(); // lhs +|- rhs = res
        let is_mul_div= common.alloc_bit_value();  // lhs *|/ rhs = res

        /* 1 for lhs op rhs = res
         * 0 for lhs = res op rhs + remainder
         */
        let mode = common.alloc_bit_value();

        let is_32bits = common.alloc_bit_value();
        let is_64bits = common.alloc_bit_value();

        constraint_builder.push(
            "32 or 64",
            Box::new(move |meta| {
                vec![(is_32bits.expr(meta) + is_64bits.expr(meta) - constant_from!(1))]
            }),
        );


        constraint_builder.push(
            "binop constraints",
            Box::new(move |meta| {
                // either 1 or -1
                let add_sign = mode.expr(meta) * constant_from!(2) - constant_from!(1);
                let modulus = constant!(bn_to_field(&(BigUint::from(1u64) << 32usize)))
                    * is_32bits.expr(meta)
                    + constant!(bn_to_field(&(BigUint::from(1u64) << 64usize)))
                        * is_64bits.expr(meta);

                vec![
                    (add_sign.clone() * lhs.expr(meta) + rhs.expr(meta)
                        - add_sign * res.expr(meta)
                        - overflow.expr(meta) * modulus)
                        * is_add_sub.expr(meta),

                ]
            }),
        );

        constraint_builder.push(
            "mul constraints",
            Box::new(move |meta| {
                /* 1 for lhs op rhs = res
                 * 0 for lhs = res op rhs + remainder
                 */
                let op_mul = mode.expr(meta);
                let op_div = constant_from!(1) - mode.expr(meta);
                let modulus = constant!(bn_to_field(&(BigUint::from(1u64) << 32usize)))
                    * is_32bits.expr(meta)
                    + constant!(bn_to_field(&(BigUint::from(1u64) << 64usize)))
                        * is_64bits.expr(meta);

                vec![
                    (lhs.expr(meta) * rhs.expr(meta)
                        - res.expr(meta)
                        - overflow.expr(meta) * modulus)
                        * is_mul_div.expr(meta) * op_mul.clone(),
                ]
            }),
        );

        constraint_builder.push(
            "div_u constraints",
            Box::new(move |meta| {
                /* 1 for lhs op rhs = res
                 * 0 for lhs = res op rhs + remainder
                 */
                let op_div = constant_from!(1) - mode.expr(meta);
                vec![
                    (lhs.expr(meta)
                        - rhs.expr(meta) * res.expr(meta)
                        - r.expr(meta)) * op_div.clone()
                        * is_mul_div.expr(meta),
                    (r.expr(meta) + overflow.expr(meta) - rhs.expr(meta))
                        * is_mul_div.expr(meta)
                        * op_div
                ]
            }),
        );

        let lookup_stack_read_lhs = common.alloc_mtable_lookup();
        let lookup_stack_read_rhs = common.alloc_mtable_lookup();
        let lookup_stack_write = common.alloc_mtable_lookup();

        Box::new(BinConfig {
            lhs,
            rhs,
            res,
            overflow,
            vtype,
            is_add_sub,
            is_mul_div,
            mode,
            r,
            is_32bits,
            is_64bits,
            lookup_stack_read_lhs,
            lookup_stack_read_rhs,
            lookup_stack_write,
        })
    }
}

impl<F: FieldExt> EventTableOpcodeConfig<F> for BinConfig {
    fn opcode(&self, meta: &mut VirtualCells<'_, F>) -> Expression<F> {
        constant!(bn_to_field(
            &(BigUint::from(OpcodeClass::Bin as u64) << OPCODE_CLASS_SHIFT)
        )) + (self.is_mul_div.expr(meta) * constant_from!(2) + constant_from!(1) - self.mode.expr(meta))
            * constant!(bn_to_field(&(BigUint::from(1u64) << OPCODE_ARG0_SHIFT)))
            + self.vtype.expr(meta)
                * constant!(bn_to_field(&(BigUint::from(1u64) << OPCODE_ARG1_SHIFT)))
    }

    fn assign(
        &self,
        ctx: &mut Context<'_, F>,
        step_info: &StepStatus,
        entry: &EventTableEntry,
    ) -> Result<(), Error> {
        let (class, vtype, shift, left, right, value) = match &entry.step_info {
            StepInfo::I32BinOp {
                class,
                left,
                right,
                value,
            } => {
                let vtype = VarType::I32;
                let left = *left as u32 as u64;
                let right = *right as u32 as u64;
                let value = *value as u32 as u64;

                (class, vtype, 32, left, right, value)
            }

            StepInfo::I64BinOp {
                class,
                left,
                right,
                value,
            } => {
                let vtype = VarType::I64;
                let left = *left as u64;
                let right = *right as u64;
                let value = *value as u64;

                (class, vtype, 64, left, right, value)
            }

            _ => unreachable!(),
        };

        self.vtype.assign(ctx, vtype as u16)?;
        self.lhs.assign(ctx, left)?;
        self.rhs.assign(ctx, right)?;
        self.res.assign(ctx, value)?;

        match class {
            specs::itable::BinOp::Add => {
                self.is_add_sub.assign(ctx, true)?;
                self.is_mul_div.assign(ctx, false)?;
                self.mode.assign(ctx, true)?;
                self.overflow.assign(ctx,
                    ((BigUint::from(left) + BigUint::from(right)) >> shift as u64).to_u64().unwrap()
                )?;
                self.r.assign(ctx, 0)?;
            },
            specs::itable::BinOp::Sub=> {
                self.is_add_sub.assign(ctx, true)?;
                self.is_mul_div.assign(ctx, false)?;
                self.mode.assign(ctx, false)?;
                self.r.assign(ctx, 0)?;
                self.overflow.assign(ctx,
                    ((BigUint::from(value) + BigUint::from(right)) >> shift as u64).to_u64().unwrap()
                )?;
            },
            specs::itable::BinOp::Mul => {
                self.is_add_sub.assign(ctx, false)?;
                self.is_mul_div.assign(ctx, true)?;
                self.mode.assign(ctx, true)?;
                self.overflow.assign(ctx,
                    ((BigUint::from(left) * BigUint::from(right)) >> shift as u64).to_u64().unwrap()
                )?;
                self.r.assign(ctx, 0)?;
            },
            specs::itable::BinOp::Div => {
                self.is_add_sub.assign(ctx, false)?;
                self.is_mul_div.assign(ctx, true)?;
                self.mode.assign(ctx, false)?;
                self.overflow.assign(ctx, (value + 1) * right - left)?;
                self.r.assign(ctx, left - value * right)?;
            },

        };

        match vtype {
            VarType::I32 => self.is_32bits.assign(ctx, true)?,
            VarType::I64 => self.is_64bits.assign(ctx, true)?,
            _ => unreachable!(),
        };

        self.lookup_stack_read_lhs.assign(
            ctx,
            &MemoryTableLookupEncode::encode_stack_read(
                BigUint::from(step_info.current.eid),
                BigUint::from(1 as u64),
                BigUint::from(step_info.current.sp + 1),
                BigUint::from(vtype as u16),
                BigUint::from(right),
            ),
        )?;

        self.lookup_stack_read_rhs.assign(
            ctx,
            &MemoryTableLookupEncode::encode_stack_read(
                BigUint::from(step_info.current.eid),
                BigUint::from(2 as u64),
                BigUint::from(step_info.current.sp + 2),
                BigUint::from(vtype as u16),
                BigUint::from(left),
            ),
        )?;

        self.lookup_stack_write.assign(
            ctx,
            &MemoryTableLookupEncode::encode_stack_write(
                BigUint::from(step_info.current.eid),
                BigUint::from(3 as u64),
                BigUint::from(step_info.current.sp + 2),
                BigUint::from(vtype as u16),
                BigUint::from(value),
            ),
        )?;

        Ok(())
    }

    fn opcode_class(&self) -> OpcodeClass {
        OpcodeClass::Bin
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
            _ => None,
        }
    }

    fn sp_diff(&self, _meta: &mut VirtualCells<'_, F>) -> Option<Expression<F>> {
        Some(constant!(F::one()))
    }
}

#[cfg(test)]
mod tests {
    use crate::test::test_circuit_noexternal;

    #[test]
    fn test_i32_add() {
        let textual_repr = r#"
                (module
                    (func (export "test")
                      (i32.const 1)
                      (i32.const 1)
                      i32.add
                      drop
                    )
                   )
                "#;

        test_circuit_noexternal(textual_repr).unwrap()
    }

    #[test]
    fn test_i32_add_overflow() {
        let textual_repr = r#"
                (module
                    (func (export "test")
                      (i32.const 1)
                      (i32.const 4294967295)
                      i32.add
                      drop
                    )
                   )
                "#;

        test_circuit_noexternal(textual_repr).unwrap()
    }

    #[test]
    fn test_i64_add() {
        let textual_repr = r#"
                (module
                    (func (export "test")
                      (i64.const 1)
                      (i64.const 1)
                      i64.add
                      drop
                    )
                   )
                "#;

        test_circuit_noexternal(textual_repr).unwrap()
    }

    #[test]
    fn test_i64_add_overflow() {
        let textual_repr = r#"
                (module
                    (func (export "test")
                      (i64.const 1)
                      (i64.const 18446744073709551615)
                      i64.add
                      drop
                    )
                   )
                "#;

        test_circuit_noexternal(textual_repr).unwrap()
    }

    #[test]
    fn test_i32_sub() {
        let textual_repr = r#"
                (module
                    (func (export "test")
                      (i32.const 1)
                      (i32.const 1)
                      i32.sub
                      drop
                    )
                   )
                "#;

        test_circuit_noexternal(textual_repr).unwrap()
    }

    #[test]
    fn test_i32_sub_overflow() {
        let textual_repr = r#"
                (module
                    (func (export "test")
                      (i32.const 0)
                      (i32.const 1)
                      i32.sub
                      drop
                    )
                   )
                "#;

        test_circuit_noexternal(textual_repr).unwrap()
    }

    #[test]
    fn test_i64_sub() {
        let textual_repr = r#"
                (module
                    (func (export "test")
                      (i64.const 1)
                      (i64.const 1)
                      i64.sub
                      drop
                    )
                   )
                "#;

        test_circuit_noexternal(textual_repr).unwrap()
    }

    #[test]
    fn test_i64_sub_overflow() {
        let textual_repr = r#"
                (module
                    (func (export "test")
                      (i64.const 0)
                      (i64.const 1)
                      i64.sub
                      drop
                    )
                   )
                "#;

        test_circuit_noexternal(textual_repr).unwrap()
    }

    #[test]
    fn test_i32_mult() {
        let textual_repr = r#"
                (module
                    (func (export "test")
                      (i32.const 4)
                      (i32.const 3)
                      i32.mul
                      drop
                    )
                   )
                "#;

        test_circuit_noexternal(textual_repr).unwrap()
    }

    #[test]
    fn test_i32_mult_overflow() {
        let textual_repr = r#"
                (module
                    (func (export "test")
                      (i32.const 4294967295)
                      (i32.const 4294967295)
                      i32.mul
                      drop
                    )
                   )
                "#;

        test_circuit_noexternal(textual_repr).unwrap()
    }

    #[test]
    fn test_i32_divu_normal() {
        let textual_repr = r#"
                (module
                    (func (export "test")
                      (i32.const 4)
                      (i32.const 3)
                      i32.div_u
                      drop
                    )
                   )
                "#;

        test_circuit_noexternal(textual_repr).unwrap()
    }


    #[test]
    fn test_i32_divu_zero() {
        let textual_repr = r#"
                (module
                    (func (export "test")
                      (i32.const 4)
                      (i32.const 4)
                      i32.div_u
                      drop
                    )
                   )
                "#;

        test_circuit_noexternal(textual_repr).unwrap()
    }

    #[test]
    fn test_i64_mult() {
        let textual_repr = r#"
                (module
                    (func (export "test")
                      (i64.const 4)
                      (i64.const 3)
                      i64.mul
                      drop
                    )
                   )
                "#;

        test_circuit_noexternal(textual_repr).unwrap()
    }

    #[test]
    fn test_i64_mult_overflow() {
        let textual_repr = r#"
                (module
                    (func (export "test")
                      (i64.const 18446744073709551615)
                      (i64.const 18446744073709551615)
                      i64.mul
                      drop
                    )
                   )
                "#;

        test_circuit_noexternal(textual_repr).unwrap()
    }

    #[test]
    fn test_i64_divu_normal() {
        let textual_repr = r#"
                (module
                    (func (export "test")
                      (i64.const 4)
                      (i64.const 3)
                      i64.div_u
                      drop
                    )
                   )
                "#;

        test_circuit_noexternal(textual_repr).unwrap()
    }


    #[test]
    fn test_i64_divu_zero() {
        let textual_repr = r#"
                (module
                    (func (export "test")
                      (i64.const 4)
                      (i64.const 4)
                      i64.div_u
                      drop
                    )
                   )
                "#;

        test_circuit_noexternal(textual_repr).unwrap()
    }

}
