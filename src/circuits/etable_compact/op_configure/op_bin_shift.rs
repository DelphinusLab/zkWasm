use super::*;
use crate::{
    circuits::{
        mtable_compact::encode::MemoryTableLookupEncode,
        rtable::pow_table_encode,
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
    itable::{OPCODE_ARG0_SHIFT, OPCODE_CLASS_SHIFT},
};

pub struct BinShiftConfig {
    lhs: U64Cell,
    rhs: U64OnU8Cell,
    modulus: U64Cell, // modulus = 2 ^ (rhs % REM_OF_SIZE)
    round: U64OnU8Cell,
    rem: U64Cell,      // round * x + rem = y
    diff: U64OnU8Cell, // diff + rem + 1 = x
    pad: U64OnU8Cell,  // the padding part when doing signed op
    res: UnlimitedCell,

    rhs_round: CommonRangeCell,
    rhs_rem: CommonRangeCell,
    rhs_rem_diff: CommonRangeCell,

    flag_bit: BitCell,
    flag_u4_rem: CommonRangeCell,
    flag_u4_rem_diff: CommonRangeCell,

    is_eight_bytes: BitCell,

    is_shl: BitCell,
    is_shr_u: BitCell,
    is_shr_s: BitCell,
    is_rotl: BitCell,
    is_rotr: BitCell,

    lookup_pow: PowTableLookupCell,

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
        let rhs = common.alloc_u64_on_u8();
        let modulus = common.alloc_u64();
        let round = common.alloc_u64_on_u8();
        let diff = common.alloc_u64_on_u8();
        let rem = common.alloc_u64();
        let res = common.alloc_unlimited_value();

        let is_eight_bytes = common.alloc_bit_value();

        let rhs_round = common.alloc_common_range_value();
        let rhs_rem = common.alloc_common_range_value();
        let rhs_rem_diff = common.alloc_common_range_value();

        let flag_u4_rem = common.alloc_common_range_value();
        let flag_u4_rem_diff = common.alloc_common_range_value();

        let is_shl = common.alloc_bit_value();
        let is_shr_u = common.alloc_bit_value();
        let is_shr_s = common.alloc_bit_value();
        let is_rotl = common.alloc_bit_value();
        let is_rotr = common.alloc_bit_value();
        let flag_bit = common.alloc_bit_value();
        let pad = common.alloc_u64_on_u8();
        let lookup_pow = common.alloc_pow_table_lookup();

        let lookup_stack_read_lhs = common.alloc_mtable_lookup();
        let lookup_stack_read_rhs = common.alloc_mtable_lookup();
        let lookup_stack_write = common.alloc_mtable_lookup();

        constraint_builder.push(
            "bin op select",
            Box::new(move |meta| {
                vec![
                    is_shr_u.expr(meta)
                        + is_shr_s.expr(meta)
                        + is_shl.expr(meta)
                        + is_rotl.expr(meta)
                        + is_rotr.expr(meta)
                        - constant_from!(1),
                ]
            }),
        );

        constraint_builder.push(
            "bin rhs rem",
            Box::new(move |meta| {
                let bit_modulus =
                    is_eight_bytes.expr(meta) * constant_from!(32) + constant_from!(32);
                vec![
                    rhs_round.expr(meta) * bit_modulus.clone() + rhs_rem.expr(meta)
                        - rhs.u8_expr(meta, 0),
                    rhs_rem.expr(meta) + rhs_rem_diff.expr(meta) + constant_from!(1) - bit_modulus,
                ]
            }),
        );

        constraint_builder.push(
            "bin lhs flag bit",
            Box::new(move |meta| {
                let flag_u4 = is_eight_bytes.expr(meta) * lhs.u4_expr(meta, 15)
                    + (constant_from!(1) - is_eight_bytes.expr(meta)) * lhs.u4_expr(meta, 7);
                vec![
                    flag_bit.expr(meta) * constant_from!(8) + flag_u4_rem.expr(meta) - flag_u4,
                    flag_u4_rem.expr(meta) + flag_u4_rem_diff.expr(meta) - constant_from!(7),
                ]
            }),
        );

        constraint_builder.push(
            "bin modulus pow lookup",
            Box::new(move |meta| {
                vec![
                    lookup_pow.expr(meta)
                        - pow_table_encode(modulus.expr(meta), rhs_rem.expr(meta)),
                ]
            }),
        );

        constraint_builder.push(
            "bin shr_u",
            Box::new(move |meta| {
                vec![
                    is_shr_u.expr(meta)
                        * (rem.expr(meta) + diff.expr(meta) + constant_from!(1)
                            - modulus.expr(meta)),
                    is_shr_u.expr(meta)
                        * (rem.expr(meta) + round.expr(meta) * modulus.expr(meta) - lhs.expr(meta)),
                    is_shr_u.expr(meta) * (res.expr(meta) - round.expr(meta)),
                ]
            }),
        );

        constraint_builder.push(
            "bin shr_s",
            Box::new(move |meta| {
                let size_modulus = is_eight_bytes.expr(meta)
                    * constant_from!((u32::MAX as u64) << 32)
                    + constant_from!(1u64 << 32);
                vec![
                    is_shr_s.expr(meta)
                        * (rem.expr(meta) + round.expr(meta) * modulus.expr(meta) - lhs.expr(meta)),
                    is_shr_s.expr(meta)
                        * (rem.expr(meta) + diff.expr(meta) + constant_from!(1)
                            - modulus.expr(meta)),
                    is_shr_s.expr(meta) * (res.expr(meta) - round.expr(meta) - pad.expr(meta)),
                    is_shr_s.expr(meta)
                        * (pad.expr(meta) * modulus.expr(meta)
                            - flag_bit.expr(meta)
                                * (modulus.expr(meta) - constant_from!(1))
                                * size_modulus),
                ]
            }),
        );

        constraint_builder.push(
            "bin shl",
            Box::new(move |meta| {
                let size_modulus = is_eight_bytes.expr(meta)
                    * constant_from!((u32::MAX as u64) << 32)
                    + constant_from!(1u64 << 32);
                vec![
                    is_shl.expr(meta)
                        * (lhs.expr(meta) * modulus.expr(meta)
                            - round.expr(meta) * size_modulus.clone()
                            - rem.expr(meta)),
                    is_shl.expr(meta)
                        * (rem.expr(meta) + diff.expr(meta) + constant_from!(1) - size_modulus),
                    is_shl.expr(meta) * (res.expr(meta) - rem.expr(meta)),
                ]
            }),
        );

        constraint_builder.push(
            "bin rotl",
            Box::new(move |meta| {
                let size_modulus = is_eight_bytes.expr(meta)
                    * constant_from!((u32::MAX as u64) << 32)
                    + constant_from!(1u64 << 32);
                vec![
                    is_rotl.expr(meta)
                        * (lhs.expr(meta) * modulus.expr(meta)
                            - round.expr(meta) * size_modulus.clone()
                            - rem.expr(meta)),
                    is_rotl.expr(meta)
                        * (rem.expr(meta) + diff.expr(meta) + constant_from!(1) - size_modulus),
                    is_rotl.expr(meta) * (res.expr(meta) - rem.expr(meta) - round.expr(meta)),
                ]
            }),
        );
        constraint_builder.push(
            "bin rotr",
            Box::new(move |meta| {
                let size_modulus = is_eight_bytes.expr(meta)
                    * constant_from!((u32::MAX as u64) << 32)
                    + constant_from!(1u64 << 32);
                vec![
                    is_rotr.expr(meta)
                        * (rem.expr(meta) + diff.expr(meta) + constant_from!(1u64)
                            - modulus.expr(meta)),
                    is_rotr.expr(meta)
                        * (rem.expr(meta) + round.expr(meta) * modulus.expr(meta) - lhs.expr(meta)),
                    is_rotr.expr(meta)
                        * (res.expr(meta) * modulus.expr(meta)
                            - round.expr(meta) * modulus.expr(meta)
                            - rem.expr(meta) * size_modulus),
                ]
            }),
        );

        Box::new(BinShiftConfig {
            lhs,
            rhs,
            modulus,
            round,
            rem,
            diff,
            pad,
            res,
            is_eight_bytes,
            is_shl,
            is_shr_u,
            is_shr_s,
            is_rotl,
            is_rotr,
            flag_bit,
            lookup_pow,
            lookup_stack_read_lhs,
            lookup_stack_read_rhs,
            lookup_stack_write,
            rhs_round,
            rhs_rem,
            rhs_rem_diff,
            flag_u4_rem,
            flag_u4_rem_diff,
        })
    }
}

impl<F: FieldExt> EventTableOpcodeConfig<F> for BinShiftConfig {
    fn opcode(&self, meta: &mut VirtualCells<'_, F>) -> Expression<F> {
        let vtype = self.is_eight_bytes.expr(meta) + constant_from!(1);

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
            + self.is_shr_s.expr(meta)
                * constant!(bn_to_field(
                    &(BigUint::from(ShiftOp::SignedShr as u64) << OPCODE_ARG0_SHIFT)
                ))
            + self.is_rotl.expr(meta)
                * constant!(bn_to_field(
                    &(BigUint::from(ShiftOp::Rotl as u64) << OPCODE_ARG0_SHIFT)
                ))
            + self.is_rotr.expr(meta)
                * constant!(bn_to_field(
                    &(BigUint::from(ShiftOp::Rotr as u64) << OPCODE_ARG0_SHIFT)
                ))
            + vtype * constant!(bn_to_field(&(BigUint::from(1u64) << OPCODE_ARG1_SHIFT)))
    }

    fn assign(
        &self,
        ctx: &mut Context<'_, F>,
        step_info: &StepStatus,
        entry: &EventTableEntry,
    ) -> Result<(), Error> {
        let (class, vtype, left, right, value, power, is_eight_bytes, _is_sign) =
            match entry.step_info {
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
                    let power = right % 32;
                    let is_eight_bytes = false;
                    let is_sign = true;
                    (
                        class,
                        vtype,
                        left,
                        right,
                        value,
                        power,
                        is_eight_bytes,
                        is_sign,
                    )
                }

                StepInfo::I64BinShiftOp {
                    class,
                    left,
                    right,
                    value,
                } => {
                    // FIXME: check
                    let vtype = VarType::I64;
                    let left = left as u64;
                    let right = right as u64;
                    let value = value as u64;
                    let power = right % 64;
                    let is_eight_bytes = true;
                    let is_sign = true;
                    (
                        class,
                        vtype,
                        left,
                        right,
                        value,
                        power,
                        is_eight_bytes,
                        is_sign,
                    )
                }

                _ => {
                    unreachable!();
                }
            };

        let size = if is_eight_bytes { 64 } else { 32 };
        let size_mask = if is_eight_bytes {
            u64::MAX
        } else {
            u32::MAX as u64
        };

        self.lhs.assign(ctx, left)?;
        let flag_u4 = (left >> (size - 4)) as u64;
        let flag_bit = flag_u4 >> 3;
        self.flag_bit.assign(ctx, flag_bit == 1)?;
        self.flag_u4_rem.assign(ctx, F::from(flag_u4 & 7))?;
        self.flag_u4_rem_diff
            .assign(ctx, F::from(7 - (flag_u4 & 7)))?;
        self.rhs.assign(ctx, right)?;
        self.rhs_round.assign(ctx, F::from((right & 0xff) / size))?;
        self.rhs_rem.assign(ctx, F::from(power))?;
        self.rhs_rem_diff.assign(ctx, F::from(size - 1 - power))?;
        self.modulus.assign(ctx, 1 << power)?;
        self.lookup_pow.assign(ctx, power)?;
        self.is_eight_bytes.assign(ctx, is_eight_bytes)?;
        self.res.assign(ctx, F::from(value))?;

        match class {
            ShiftOp::Shl => {
                self.is_shl.assign(ctx, true)?;
                if power != 0 {
                    self.round.assign(ctx, left >> (size - power))?;
                } else {
                    self.round.assign(ctx, 0)?;
                }
                let rem = (left << power) & size_mask;
                self.rem.assign(ctx, rem)?;
                self.diff.assign(ctx, size_mask - rem)?;
            }
            ShiftOp::UnsignedShr => {
                self.is_shr_u.assign(ctx, true)?;
                self.round.assign(ctx, left >> power)?;
                let rem = left & ((1 << power) - 1);
                self.rem.assign(ctx, rem)?;
                self.diff.assign(ctx, (1u64 << power) - rem - 1)?;
            }
            ShiftOp::SignedShr => {
                self.is_shr_s.assign(ctx, true)?;
                self.round.assign(ctx, left >> power)?;
                let rem = left & ((1 << power) - 1);
                self.rem.assign(ctx, rem)?;
                self.diff.assign(ctx, (1u64 << power) - 1 - rem)?;

                if flag_bit == 1 && power != 0 {
                    self.pad.assign(ctx, ((1 << power) - 1) << (size - power))?;
                }
            }
            ShiftOp::Rotl => {
                // same as shl
                self.is_rotl.assign(ctx, true)?;
                if power != 0 {
                    self.round.assign(ctx, left >> (size - power))?;
                } else {
                    self.round.assign(ctx, 0)?;
                }
                let rem = (left << power) & size_mask;
                self.rem.assign(ctx, rem)?;
                self.diff.assign(ctx, size_mask - rem)?;
            }
            ShiftOp::Rotr => {
                // same as shr_u
                self.is_rotr.assign(ctx, true)?;
                self.round.assign(ctx, left >> power)?;
                let rem = left & ((1 << power) - 1);
                self.rem.assign(ctx, rem)?;
                self.diff.assign(ctx, (1u64 << power) - rem - 1)?;
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

    fn mops(&self, _meta: &mut VirtualCells<'_, F>) -> Option<Expression<F>> {
        Some(constant_from!(3))
    }

    fn mtable_lookup(
        &self,
        meta: &mut VirtualCells<'_, F>,
        item: MLookupItem,
        common_config: &EventTableCommonConfig<F>,
    ) -> Option<Expression<F>> {
        let vtype = self.is_eight_bytes.expr(meta) + constant_from!(1);
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
    fn test_bin_shift_rhs_0_ok() {
        let textual_repr = r#"
                (module
                    (func (export "test")
                      (i32.const 12)
                      (i32.const 0)
                      (i32.shl)
                      (drop)
                      (i32.const 12)
                      (i32.const 0)
                      (i32.shr_u)
                      (drop)
                      (i32.const 12)
                      (i32.const 0)
                      (i32.shr_s)
                      (drop)
                      (i32.const 12)
                      (i32.const 0)
                      (i32.rotl)
                      (drop)
                      (i32.const 12)
                      (i32.const 0)
                      (i32.rotr)
                      (drop)

                      (i64.const 12)
                      (i64.const 0)
                      (i64.shl)
                      (drop)
                      (i64.const 12)
                      (i64.const 0)
                      (i64.shr_u)
                      (drop)
                      (i64.const 12)
                      (i64.const 0)
                      (i64.shr_s)
                      (drop)
                      (i64.const 12)
                      (i64.const 0)
                      (i64.rotl)
                      (drop)
                      (i64.const 12)
                      (i64.const 0)
                      (i64.rotr)
                      (drop)
                    )
                   )
                "#;

        test_circuit_noexternal(textual_repr).unwrap()
    }

    #[test]
    fn test_i32_shl_ok() {
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
    fn test_i32_shl_rhs_overflow_ok() {
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
    fn test_i32_shl_res_overflow_ok() {
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

    #[test]
    fn test_i32_shr_s_positive() {
        let textual_repr = r#"
                (module
                    (func (export "test")
                      (i32.const 23)
                      (i32.const 2)
                      (i32.shr_s)
                      (drop)
                    )
                   )
                "#;

        test_circuit_noexternal(textual_repr).unwrap()
    }

    #[test]
    fn test_i32_shr_s_positive_rhs_overflow() {
        let textual_repr = r#"
                (module
                    (func (export "test")
                      (i32.const 23)
                      (i32.const 35)
                      (i32.shr_s)
                      (drop)
                    )
                   )
                "#;
        test_circuit_noexternal(textual_repr).unwrap()
    }

    #[test]
    fn test_i32_shr_s_negative() {
        let textual_repr = r#"
                (module
                    (func (export "test")
                      (i32.const -23)
                      (i32.const 5)
                      (i32.shr_s)
                      (drop)
                    )
                   )
                "#;

        test_circuit_noexternal(textual_repr).unwrap()
    }

    #[test]
    fn test_i32_shr_s_negative_rhs_overflow() {
        let textual_repr = r#"
                (module
                    (func (export "test")
                      (i32.const -23)
                      (i32.const 35)
                      (i32.shr_s)
                      (drop)
                    )
                   )
                "#;

        test_circuit_noexternal(textual_repr).unwrap()
    }
    #[test]
    fn test_i32_shr_s_zero() {
        let textual_repr = r#"
                (module
                    (func (export "test")
                      (i32.const 0)
                      (i32.const 5)
                      (i32.shr_s)
                      (drop)
                    )
                   )
                "#;

        test_circuit_noexternal(textual_repr).unwrap()
    }
    #[test]
    fn test_i32_shr_s_res_overflow() {
        let textual_repr = r#"
                (module
                    (func (export "test")
                      (i32.const -1)
                      (i32.const 5)
                      (i32.shr_s)
                      (drop)
                    )
                   )
                "#;

        test_circuit_noexternal(textual_repr).unwrap()
    }
    #[test]
    fn test_i32_rotl() {
        let textual_repr = r#"
                (module
                    (func (export "test")
                      (i32.const 23)
                      (i32.const 5)
                      (i32.rotl)
                      (drop)
                    )
                   )
                "#;

        test_circuit_noexternal(textual_repr).unwrap()
    }
    #[test]
    fn test_i32_rotl2() {
        let textual_repr = r#"
                (module
                    (func (export "test")
                      (i32.const 2863311530)
                      (i32.const 5)
                      (i32.rotl)
                      (drop)
                    )
                   )
                "#;

        test_circuit_noexternal(textual_repr).unwrap()
    }

    #[test]
    fn test_i32_rotr() {
        let textual_repr = r#"
                (module
                    (func (export "test")
                      (i32.const 23)
                      (i32.const 5)
                      (i32.rotr)
                      (drop)
                    )
                   )
                "#;

        test_circuit_noexternal(textual_repr).unwrap()
    }

    #[test]
    fn test_i64_shr_u_1_ok() {
        let textual_repr = r#"
                (module
                    (func (export "test")
                      (i64.const 12)
                      (i64.const 3)
                      (i64.shr_u)
                      (drop)
                    )
                   )
                "#;

        test_circuit_noexternal(textual_repr).unwrap()
    }

    #[test]
    fn test_i64_shr_u_rhs_overflow_ok() {
        let textual_repr = r#"
                (module
                    (func (export "test")
                      (i64.const 12)
                      (i64.const 68)
                      (i64.shr_u)
                      (drop)
                    )
                   )
                "#;

        test_circuit_noexternal(textual_repr).unwrap()
    }

    #[test]
    fn test_i64_rhs_zero_ok() {
        let textual_repr = r#"
                (module
                    (func (export "test")
                      (i64.const 12)
                      (i64.const 0)
                      (i64.shl)
                      (drop)
                    )
                   )
                "#;

        test_circuit_noexternal(textual_repr).unwrap()
    }

    #[test]
    fn test_i64_shl_ok() {
        let textual_repr = r#"
                (module
                    (func (export "test")
                      (i64.const 12)
                      (i64.const 1)
                      (i64.shl)
                      (drop)
                    )
                   )
                "#;

        test_circuit_noexternal(textual_repr).unwrap()
    }

    #[test]
    fn test_i64_shl_rhs_overflow_ok() {
        let textual_repr = r#"
                (module
                    (func (export "test")
                      (i64.const 12)
                      (i64.const 67)
                      (i64.shl)
                      (drop)
                    )
                   )
                "#;

        test_circuit_noexternal(textual_repr).unwrap()
    }

    #[test]
    fn test_i64_shl_res_overflow_ok() {
        let textual_repr = r#"
                (module
                    (func (export "test")
                      (i64.const 0xffffffffffffffff)
                      (i64.const 1)
                      (i64.shl)
                      (drop)
                    )
                   )
                "#;

        test_circuit_noexternal(textual_repr).unwrap()
    }

    #[test]
    fn test_i64_shr_s_positive() {
        let textual_repr = r#"
                (module
                    (func (export "test")
                      (i64.const 23)
                      (i64.const 2)
                      (i64.shr_s)
                      (drop)
                    )
                   )
                "#;

        test_circuit_noexternal(textual_repr).unwrap()
    }

    #[test]
    fn test_i64_shr_s_positive_rhs_overflow() {
        let textual_repr = r#"
                (module
                    (func (export "test")
                      (i64.const 23)
                      (i64.const 68)
                      (i64.shr_s)
                      (drop)
                    )
                   )
                "#;
        test_circuit_noexternal(textual_repr).unwrap()
    }

    #[test]
    fn test_i64_shr_s_negative() {
        let textual_repr = r#"
                (module
                    (func (export "test")
                      (i64.const -23)
                      (i64.const 5)
                      (i64.shr_s)
                      (drop)
                    )
                   )
                "#;

        test_circuit_noexternal(textual_repr).unwrap()
    }

    #[test]
    fn test_i64_shr_s_negative_rhs_overflow() {
        let textual_repr = r#"
                (module
                    (func (export "test")
                      (i64.const -23)
                      (i64.const 68)
                      (i64.shr_s)
                      (drop)
                    )
                   )
                "#;

        test_circuit_noexternal(textual_repr).unwrap()
    }
    #[test]
    fn test_i64_shr_s_zero() {
        let textual_repr = r#"
                (module
                    (func (export "test")
                      (i64.const 0)
                      (i64.const 5)
                      (i64.shr_s)
                      (drop)
                    )
                   )
                "#;

        test_circuit_noexternal(textual_repr).unwrap()
    }
    #[test]
    fn test_i64_rotl() {
        let textual_repr = r#"
                (module
                    (func (export "test")
                      (i64.const 23)
                      (i64.const 5)
                      (i64.rotl)
                      (drop)
                    )
                   )
                "#;

        test_circuit_noexternal(textual_repr).unwrap()
    }
    #[test]
    fn test_i64_rotl2() {
        let textual_repr = r#"
                (module
                    (func (export "test")
                      (i64.const 2863311530)
                      (i64.const 5)
                      (i64.rotl)
                      (drop)
                    )
                   )
                "#;

        test_circuit_noexternal(textual_repr).unwrap()
    }

    #[test]
    fn test_i64_rotr() {
        let textual_repr = r#"
                (module
                    (func (export "test")
                      (i64.const 23)
                      (i64.const 5)
                      (i64.rotr)
                      (drop)
                    )
                   )
                "#;

        test_circuit_noexternal(textual_repr).unwrap()
    }
}
