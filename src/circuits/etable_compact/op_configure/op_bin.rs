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

    d: U64Cell,

    res: U64OnU8Cell,
    aux1: U64OnU8Cell,
    aux2: U64OnU8Cell,
    aux3: U64OnU8Cell,

    lhs_flag: BitCell,
    lhs_flag_helper: CommonRangeCell,
    lhs_flag_helper_diff: CommonRangeCell,

    rhs_flag: BitCell,
    rhs_flag_helper: CommonRangeCell,
    rhs_flag_helper_diff: CommonRangeCell,

    d_flag_helper_diff: CommonRangeCell,

    overflow: BitCell,
    is_add: BitCell,
    is_sub: BitCell,
    is_mul: BitCell,
    is_div_u: BitCell,
    is_rem_u: BitCell,
    is_div_s: BitCell,
    is_rem_s: BitCell,
    is_64bits: BitCell,

    lookup_stack_read_lhs: MTableLookupCell,
    lookup_stack_read_rhs: MTableLookupCell,
    lookup_stack_write: MTableLookupCell,
}

pub struct BinConfigBuilder {}

impl<F: FieldExt> EventTableOpcodeConfigBuilder<F> for BinConfigBuilder {
    fn configure(
        common: &mut EventTableCellAllocator<F>,
        constraint_builder: &mut ConstraintBuilder<F>,
    ) -> Box<dyn EventTableOpcodeConfig<F>> {
        let lhs = common.alloc_u64();
        let rhs = common.alloc_u64();

        let d = common.alloc_u64();
        let aux1 = common.alloc_u64_on_u8();
        let aux2 = common.alloc_u64_on_u8();
        let aux3 = common.alloc_u64_on_u8();
        let res = common.alloc_u64_on_u8();

        let overflow = common.alloc_bit_value();

        let is_add = common.alloc_bit_value();
        let is_sub = common.alloc_bit_value();
        let is_mul = common.alloc_bit_value();
        let is_div_u = common.alloc_bit_value();
        let is_rem_u = common.alloc_bit_value();
        let is_div_s = common.alloc_bit_value();
        let is_rem_s = common.alloc_bit_value();

        let lhs_flag = common.alloc_bit_value();
        let rhs_flag = common.alloc_bit_value();

        let lhs_flag_helper = common.alloc_common_range_value();
        let lhs_flag_helper_diff = common.alloc_common_range_value();
        let rhs_flag_helper = common.alloc_common_range_value();
        let rhs_flag_helper_diff = common.alloc_common_range_value();
        let d_flag_helper_diff = common.alloc_common_range_value();

        let is_64bits = common.alloc_bit_value();

        constraint_builder.push(
            "binop: selector",
            Box::new(move |meta| {
                vec![
                    (is_add.expr(meta)
                        + is_sub.expr(meta)
                        + is_mul.expr(meta)
                        + is_div_u.expr(meta)
                        + is_rem_u.expr(meta)
                        + is_div_s.expr(meta)
                        + is_rem_s.expr(meta)
                        - constant_from!(1)),
                ]
            }),
        );

        constraint_builder.push(
            "binop: add/sub constraints",
            Box::new(move |meta| {
                let modulus = constant!(bn_to_field(&(BigUint::from(1u64) << 32usize)))
                    + constant!(bn_to_field(&(BigUint::from((u32::MAX as u64) << 32usize))))
                        * is_64bits.expr(meta);

                // The range of res can be limited with vtype in mtable
                vec![
                    (lhs.expr(meta) + rhs.expr(meta)
                        - res.expr(meta)
                        - overflow.expr(meta) * modulus.clone())
                        * is_add.expr(meta),
                    (rhs.expr(meta) + res.expr(meta)
                        - lhs.expr(meta)
                        - overflow.expr(meta) * modulus)
                        * is_sub.expr(meta),
                ]
            }),
        );

        constraint_builder.push(
            "binop: mul constraints",
            Box::new(move |meta| {
                let modulus = constant!(bn_to_field(&(BigUint::from(1u64) << 32usize)))
                    + constant!(bn_to_field(&(BigUint::from((u32::MAX as u64) << 32usize))))
                        * is_64bits.expr(meta);

                // The range of res can be limited with vtype in mtable
                vec![
                    (lhs.expr(meta) * rhs.expr(meta)
                        - aux1.expr(meta) * modulus.clone()
                        - res.expr(meta))
                        * is_mul.expr(meta),
                ]
            }),
        );

        constraint_builder.push(
            "binop: div_u/rem_u constraints",
            Box::new(move |meta| {
                vec![
                    (lhs.expr(meta) - rhs.expr(meta) * aux1.expr(meta) - aux2.expr(meta))
                        * (is_rem_u.expr(meta) + is_div_u.expr(meta)),
                    (aux2.expr(meta) + aux3.expr(meta) + constant_from!(1) - rhs.expr(meta))
                        * (is_rem_u.expr(meta) + is_div_u.expr(meta)),
                    (res.expr(meta) - aux1.expr(meta)) * is_div_u.expr(meta),
                    (res.expr(meta) - aux2.expr(meta)) * is_rem_u.expr(meta),
                ]
            }),
        );

        constraint_builder.push(
            "binop: div_s/rem_s constraints common",
            Box::new(move |meta| {
                let enable = is_div_s.expr(meta) + is_rem_s.expr(meta);

                let modulus = constant!(bn_to_field(&(BigUint::from(1u64) << 32usize)))
                    + constant!(bn_to_field(&(BigUint::from((u32::MAX as u64) << 32usize))))
                        * is_64bits.expr(meta);

                let lhs_leading_u4 = lhs.u4_expr(meta, 7)
                    + (lhs.u4_expr(meta, 15) - lhs.u4_expr(meta, 7)) * is_64bits.expr(meta);
                let rhs_leading_u4 = rhs.u4_expr(meta, 7)
                    + (rhs.u4_expr(meta, 15) - rhs.u4_expr(meta, 7)) * is_64bits.expr(meta);
                let d_leading_u4 = d.u4_expr(meta, 7)
                    + (d.u4_expr(meta, 15) - d.u4_expr(meta, 7)) * is_64bits.expr(meta);

                let normalized_lhs = lhs.expr(meta) * (constant_from!(1) - lhs_flag.expr(meta))
                    + (modulus.clone() - lhs.expr(meta)) * lhs_flag.expr(meta);
                let normalized_rhs = rhs.expr(meta) * (constant_from!(1) - rhs_flag.expr(meta))
                    + (modulus.clone() - rhs.expr(meta)) * rhs_flag.expr(meta);

                let res_flag = lhs_flag.expr(meta) + rhs_flag.expr(meta)
                    - constant_from!(2) * lhs_flag.expr(meta) * rhs_flag.expr(meta);

                vec![
                    lhs_leading_u4
                        - lhs_flag.expr(meta) * constant_from!(8)
                        - lhs_flag_helper.expr(meta),
                    lhs_flag_helper.expr(meta) + lhs_flag_helper_diff.expr(meta)
                        - constant_from!(7),
                    rhs_leading_u4
                        - rhs_flag.expr(meta) * constant_from!(8)
                        - rhs_flag_helper.expr(meta),
                    rhs_flag_helper.expr(meta) + rhs_flag_helper_diff.expr(meta)
                        - constant_from!(7),
                    // d_flag must be zero if res_flag is zero
                    (d_leading_u4 + d_flag_helper_diff.expr(meta) - constant_from!(7))
                        * (constant_from!(1) - res_flag.clone()),
                    normalized_lhs - normalized_rhs.clone() * d.expr(meta) - aux1.expr(meta),
                    aux1.expr(meta) + aux2.expr(meta) + constant_from!(1) - normalized_rhs,
                ]
                .into_iter()
                .map(|x| x * enable.clone())
                .collect()
            }),
        );

        constraint_builder.push(
            "binop: div_s constraints res",
            Box::new(move |meta| {
                let modulus = constant!(bn_to_field(&(BigUint::from(1u64) << 32usize)))
                    + constant!(bn_to_field(&(BigUint::from((u32::MAX as u64) << 32usize))))
                        * is_64bits.expr(meta);

                let res_flag = lhs_flag.expr(meta) + rhs_flag.expr(meta)
                    - constant_from!(2) * lhs_flag.expr(meta) * rhs_flag.expr(meta);

                vec![
                    (res.expr(meta) - d.expr(meta))
                        * (constant_from!(1) - res_flag.clone())
                        * is_div_s.expr(meta),
                    (res.expr(meta) + d.expr(meta) - modulus.clone())
                        * (d.expr(meta) + res.expr(meta))
                        * res_flag.clone()
                        * is_div_s.expr(meta),
                ]
            }),
        );

        constraint_builder.push(
            "binop: rem_s constraints res",
            Box::new(move |meta| {
                let modulus = constant!(bn_to_field(&(BigUint::from(1u64) << 32usize)))
                    + constant!(bn_to_field(&(BigUint::from((u32::MAX as u64) << 32usize))))
                        * is_64bits.expr(meta);

                vec![
                    (res.expr(meta) - aux1.expr(meta))
                        * (constant_from!(1) - lhs_flag.expr(meta))
                        * is_rem_s.expr(meta),
                    (res.expr(meta) + aux1.expr(meta) - modulus.clone())
                        * (aux1.expr(meta) + res.expr(meta))
                        * lhs_flag.expr(meta)
                        * is_rem_s.expr(meta),
                ]
            }),
        );

        let lookup_stack_read_lhs = common.alloc_mtable_lookup();
        let lookup_stack_read_rhs = common.alloc_mtable_lookup();
        let lookup_stack_write = common.alloc_mtable_lookup();

        Box::new(BinConfig {
            lhs,
            rhs,
            d,
            res,
            aux1,
            aux2,
            aux3,
            lhs_flag,
            lhs_flag_helper,
            lhs_flag_helper_diff,
            rhs_flag,
            rhs_flag_helper,
            rhs_flag_helper_diff,
            d_flag_helper_diff,
            overflow,
            is_add,
            is_sub,
            is_mul,
            is_div_u,
            is_rem_u,
            is_div_s,
            is_rem_s,
            is_64bits,
            lookup_stack_read_lhs,
            lookup_stack_read_rhs,
            lookup_stack_write,
        })
    }
}

impl<F: FieldExt> EventTableOpcodeConfig<F> for BinConfig {
    fn opcode(&self, meta: &mut VirtualCells<'_, F>) -> Expression<F> {
        let vtype = self.is_64bits.expr(meta) + constant_from!(1);
        constant!(bn_to_field(
            &(BigUint::from(OpcodeClass::Bin as u64) << OPCODE_CLASS_SHIFT)
        )) + self.is_add.expr(meta)
            * constant!(bn_to_field(
                &(BigUint::from(BinOp::Add as u64) << OPCODE_ARG0_SHIFT)
            ))
            + self.is_sub.expr(meta)
                * constant!(bn_to_field(
                    &(BigUint::from(BinOp::Sub as u64) << OPCODE_ARG0_SHIFT)
                ))
            + self.is_mul.expr(meta)
                * constant!(bn_to_field(
                    &(BigUint::from(BinOp::Mul as u64) << OPCODE_ARG0_SHIFT)
                ))
            + self.is_div_u.expr(meta)
                * constant!(bn_to_field(
                    &(BigUint::from(BinOp::UnsignedDiv as u64) << OPCODE_ARG0_SHIFT)
                ))
            + self.is_rem_u.expr(meta)
                * constant!(bn_to_field(
                    &(BigUint::from(BinOp::UnsignedRem as u64) << OPCODE_ARG0_SHIFT)
                ))
            + self.is_div_s.expr(meta)
                * constant!(bn_to_field(
                    &(BigUint::from(BinOp::SignedDiv as u64) << OPCODE_ARG0_SHIFT)
                ))
            + self.is_rem_s.expr(meta)
                * constant!(bn_to_field(
                    &(BigUint::from(BinOp::SignedRem as u64) << OPCODE_ARG0_SHIFT)
                ))
            + vtype * constant!(bn_to_field(&(BigUint::from(1u64) << OPCODE_ARG1_SHIFT)))
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

        self.lhs.assign(ctx, left)?;
        self.rhs.assign(ctx, right)?;
        self.res.assign(ctx, value)?;

        match class {
            BinOp::Add => {
                self.is_add.assign(ctx, true)?;
                self.overflow.assign(
                    ctx,
                    (BigUint::from(left) + BigUint::from(right)) >> shift == BigUint::one(),
                )?;
            }
            BinOp::Sub => {
                self.is_sub.assign(ctx, true)?;
                self.overflow.assign(
                    ctx,
                    (BigUint::from(right) + BigUint::from(value)) >> shift == BigUint::one(),
                )?;
            }
            BinOp::Mul => {
                self.is_mul.assign(ctx, true)?;
                self.aux1
                    .assign(ctx, ((left as u128 * right as u128) >> shift) as u64)?;
            }
            BinOp::UnsignedDiv => {
                self.is_div_u.assign(ctx, true)?;
            }
            BinOp::UnsignedRem => {
                self.is_rem_u.assign(ctx, true)?;
            }
            BinOp::SignedDiv => {
                self.is_div_s.assign(ctx, true)?;
            }
            BinOp::SignedRem => {
                self.is_rem_s.assign(ctx, true)?;
            }
        };

        match class {
            BinOp::UnsignedDiv | BinOp::UnsignedRem => {
                self.aux1.assign(ctx, left / right)?;
                self.aux2.assign(ctx, left % right)?;
                self.aux3.assign(ctx, right - left % right - 1)?;
            }
            BinOp::SignedDiv | BinOp::SignedRem => {
                let left_flag = left >> (shift - 1) != 0;
                let right_flag = right >> (shift - 1) != 0;

                self.lhs_flag.assign(ctx, left_flag)?;
                self.lhs_flag_helper
                    .assign(ctx, F::from((left >> (shift - 4)) & 7))?;
                self.lhs_flag_helper_diff
                    .assign(ctx, F::from(7 - (left >> (shift - 4)) & 7))?;
                self.rhs_flag.assign(ctx, right_flag)?;
                self.rhs_flag_helper
                    .assign(ctx, F::from((right >> (shift - 4)) & 7))?;
                self.rhs_flag_helper_diff
                    .assign(ctx, F::from(7 - (right >> (shift - 4)) & 7))?;

                let mask = if shift == 32 {
                    u32::MAX as u64
                } else {
                    u64::MAX
                };
                let normalized_lhs = if left_flag { (1 + !left) & mask } else { left };
                let normalized_rhs = if right_flag {
                    (1 + !right) & mask
                } else {
                    right
                };
                let d = normalized_lhs / normalized_rhs;
                let rem = normalized_lhs % normalized_rhs;
                let d_leading_u4 = (d >> (shift - 4)) as u16;
                self.d_flag_helper_diff.assign(
                    ctx,
                    if d_leading_u4 > 7 {
                        F::from(0)
                    } else {
                        F::from(7 - d_leading_u4 as u64)
                    },
                )?;
                self.d.assign(ctx, d)?;
                self.aux1.assign(ctx, rem)?;
                self.aux2.assign(ctx, normalized_rhs - rem - 1)?;
            }
            _ => {}
        }

        match vtype {
            VarType::I64 => self.is_64bits.assign(ctx, true)?,
            _ => {}
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
        let vtype = self.is_64bits.expr(meta) + constant_from!(1);
        match item {
            MLookupItem::First => Some(MemoryTableLookupEncode::encode_stack_read(
                common_config.eid(meta),
                constant_from!(1),
                common_config.sp(meta) + constant_from!(1),
                vtype.clone(),
                self.rhs.expr(meta),
            )),
            MLookupItem::Second => Some(MemoryTableLookupEncode::encode_stack_read(
                common_config.eid(meta),
                constant_from!(2),
                common_config.sp(meta) + constant_from!(2),
                vtype.clone(),
                self.lhs.expr(meta),
            )),
            MLookupItem::Third => Some(MemoryTableLookupEncode::encode_stack_write(
                common_config.eid(meta),
                constant_from!(3),
                common_config.sp(meta) + constant_from!(2),
                vtype.clone(),
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
    fn test_i32_divu() {
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
    fn test_i32_divs() {
        let textual_repr = r#"
                (module
                    (func (export "test")
                      (i32.const 0x80000000)
                      (i32.const 1)
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
                      (i64.const 4)
                      (i64.const 4)
                      i64.div_u
                      drop
                    )
                   )
                "#;

        test_circuit_noexternal(textual_repr).unwrap()
    }

    #[test]
    fn test_i64_remu_normal() {
        let textual_repr = r#"
                (module
                    (func (export "test")
                      (i64.const 4)
                      (i64.const 3)
                      i64.rem_u
                      drop
                      (i64.const 4)
                      (i64.const 4)
                      i64.rem_u
                      drop
                    )
                   )
                "#;

        test_circuit_noexternal(textual_repr).unwrap()
    }

    #[test]
    fn test_i64_divs_normal() {
        let textual_repr = r#"
                (module
                    (func (export "test")
                      (i64.const 4)
                      (i64.const 3)
                      i64.div_s
                      drop
                      (i64.const -4)
                      (i64.const -3)
                      i64.div_s
                      drop
                    )
                   )
                "#;

        test_circuit_noexternal(textual_repr).unwrap()
    }

    #[test]
    fn test_i64_divs_neg() {
        let textual_repr = r#"
                (module
                    (func (export "test")
                      (i64.const -4)
                      (i64.const 3)
                      i64.div_s
                      drop
                      (i64.const 4)
                      (i64.const -3)
                      i64.div_s
                      drop
                      (i64.const -3)
                      (i64.const 4)
                      i64.div_s
                      drop
                      (i64.const 0x8000000000000000)
                      (i64.const 1)
                      i64.div_s
                      drop
                    )
                   )
                "#;

        test_circuit_noexternal(textual_repr).unwrap()
    }

    #[test]
    fn test_i64_rems_normal() {
        let textual_repr = r#"
                (module
                    (func (export "test")
                      (i64.const 4)
                      (i64.const 3)
                      i64.rem_s
                      drop
                      (i64.const -4)
                      (i64.const -3)
                      i64.rem_s
                      drop
                    )
                   )
                "#;

        test_circuit_noexternal(textual_repr).unwrap()
    }

    #[test]
    fn test_i64_rems_neg() {
        let textual_repr = r#"
                (module
                    (func (export "test")
                      (i64.const -4)
                      (i64.const 3)
                      i64.rem_s
                      drop
                      (i64.const 4)
                      (i64.const -3)
                      i64.rem_s
                      drop
                      (i64.const 4)
                      (i64.const -4)
                      i64.rem_s
                      drop
                      
                    )
                   )
                "#;

        test_circuit_noexternal(textual_repr).unwrap()
    }
}
