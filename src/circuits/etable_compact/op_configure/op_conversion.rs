use super::*;
use crate::circuits::{mtable_compact::encode::MemoryTableLookupEncode, utils::Context};
use halo2_proofs::{
    arithmetic::FieldExt,
    plonk::{Error, Expression, VirtualCells},
};
use specs::step::StepInfo;
use specs::{encode::opcode::encode_conversion, mtable::VarType};
use specs::{etable::EventTableEntry, itable::ConversionOp};

pub struct ConversionConfig {
    value: U64OnU8Cell,
    value_type: CommonRangeCell,
    res: U64Cell,
    res_type: CommonRangeCell,

    flag_bit: BitCell,
    flag_u8_rem: CommonRangeCell,
    flag_u8_rem_diff: CommonRangeCell,

    is_i32_wrap_i64: BitCell,
    is_i64_extend_i32_u: BitCell,
    is_i64_extend_i32_s: BitCell,

    lookup_stack_read: MTableLookupCell,
    lookup_stack_write: MTableLookupCell,
}

pub struct ConversionConfigBuilder {}

impl<F: FieldExt> EventTableOpcodeConfigBuilder<F> for ConversionConfigBuilder {
    fn configure(
        common: &mut EventTableCellAllocator<F>,
        constraint_builder: &mut ConstraintBuilder<F>,
    ) -> Box<dyn EventTableOpcodeConfig<F>> {
        let value = common.alloc_u64_on_u8();
        let value_type = common.alloc_common_range_value();

        let res = common.alloc_u64();
        let res_type = common.alloc_common_range_value();

        let flag_bit = common.alloc_bit_value();
        let flag_u8_rem = common.alloc_common_range_value();
        let flag_u8_rem_diff = common.alloc_common_range_value();

        let is_i32_wrap_i64 = common.alloc_bit_value();
        let is_i64_extend_i32_u = common.alloc_bit_value();
        let is_i64_extend_i32_s = common.alloc_bit_value();

        let lookup_stack_read = common.alloc_mtable_lookup();
        let lookup_stack_write = common.alloc_mtable_lookup();

        constraint_builder.push(
            "op_conversion pick one",
            Box::new(move |meta| {
                vec![
                    is_i32_wrap_i64.expr(meta)
                        + is_i64_extend_i32_u.expr(meta)
                        + is_i64_extend_i32_s.expr(meta)
                        - constant_from!(1),
                ]
            }),
        );

        constraint_builder.push(
            "type matches op",
            Box::new(move |meta| {
                vec![
                    is_i32_wrap_i64.expr(meta)
                        * (value_type.expr(meta) - constant_from!(VarType::I64)),
                    is_i32_wrap_i64.expr(meta)
                        * (res_type.expr(meta) - constant_from!(VarType::I32)),
                    (is_i64_extend_i32_s.expr(meta) + is_i64_extend_i32_u.expr(meta))
                        * (value_type.expr(meta) - constant_from!(VarType::I32)),
                    (is_i64_extend_i32_s.expr(meta) + is_i64_extend_i32_u.expr(meta))
                        * (res_type.expr(meta) - constant_from!(VarType::I64)),
                ]
            }),
        );

        constraint_builder.push(
            "i32_wrap_i64",
            Box::new(move |meta| {
                let mut acc = constant_from!(0);

                for i in 0..4 {
                    acc = acc + value.u8_expr(meta, i) * constant_from!(1 << (i * 8));
                }

                vec![is_i32_wrap_i64.expr(meta) * (acc - res.expr(meta))]
            }),
        );

        constraint_builder.push(
            "extend op flag bit",
            Box::new(move |meta| {
                let flag_u8 = value.u8_expr(meta, 3);
                vec![
                    (is_i64_extend_i32_s.expr(meta) + is_i64_extend_i32_u.expr(meta))
                        * (flag_bit.expr(meta) * constant_from!(128) + flag_u8_rem.expr(meta)
                            - flag_u8),
                    (is_i64_extend_i32_s.expr(meta) + is_i64_extend_i32_u.expr(meta))
                        * (flag_u8_rem.expr(meta) + flag_u8_rem_diff.expr(meta)
                            - constant_from!(127)),
                ]
            }),
        );

        constraint_builder.push(
            "i64_extend_i32_u",
            Box::new(move |meta| {
                vec![is_i64_extend_i32_u.expr(meta) * (res.expr(meta) - value.expr(meta))]
            }),
        );

        constraint_builder.push(
            "i64_extend_i32_s",
            Box::new(move |meta| {
                let pad = flag_bit.expr(meta) * constant_from!((u32::MAX as u64) << 32);

                vec![is_i64_extend_i32_s.expr(meta) * (pad + value.expr(meta) - res.expr(meta))]
            }),
        );

        Box::new(ConversionConfig {
            value,
            value_type,
            res,
            res_type,
            flag_bit,
            flag_u8_rem,
            flag_u8_rem_diff,
            is_i32_wrap_i64,
            is_i64_extend_i32_u,
            is_i64_extend_i32_s,
            lookup_stack_read,
            lookup_stack_write,
        })
    }
}

impl<F: FieldExt> EventTableOpcodeConfig<F> for ConversionConfig {
    fn opcode(&self, meta: &mut VirtualCells<'_, F>) -> Expression<F> {
        self.is_i32_wrap_i64.expr(meta)
            * encode_conversion::<Expression<F>>(ConversionOp::I32WrapI64)
            + self.is_i64_extend_i32_s.expr(meta)
                * encode_conversion::<Expression<F>>(ConversionOp::I64ExtendI32s)
            + self.is_i64_extend_i32_u.expr(meta)
                * encode_conversion::<Expression<F>>(ConversionOp::I64ExtendI32u)
    }

    fn assign(
        &self,
        ctx: &mut Context<'_, F>,
        step_info: &StepStatus,
        entry: &EventTableEntry,
    ) -> Result<(), Error> {
        let (value, value_type, result, result_type) = match entry.step_info {
            StepInfo::I32WrapI64 { value, result } => {
                let value = value as u64;

                self.is_i32_wrap_i64.assign(ctx, true)?;

                (value, VarType::I64, result as u32 as u64, VarType::I32)
            }
            StepInfo::I64ExtendI32 {
                value,
                result,
                sign,
            } => {
                if sign {
                    self.is_i64_extend_i32_s.assign(ctx, true)?;
                } else {
                    self.is_i64_extend_i32_u.assign(ctx, true)?;
                }

                let flag_u8 = value as u32 >> (32 - 8);
                let flag_bit = flag_u8 >> 7;
                let flag_u8_rem = flag_u8 & 0x7f;
                let flag_u8_rem_diff = 0x7f - flag_u8_rem;

                self.flag_bit.assign(ctx, flag_bit == 1)?;
                self.flag_u8_rem.assign(ctx, F::from(flag_u8_rem as u64))?;
                self.flag_u8_rem_diff
                    .assign(ctx, F::from(flag_u8_rem_diff as u64))?;

                (
                    value as u32 as u64,
                    VarType::I32,
                    result as u64,
                    VarType::I64,
                )
            }
            _ => unreachable!(),
        };

        self.value.assign(ctx, value)?;
        self.res.assign(ctx, result)?;
        self.value_type.assign(ctx, F::from(value_type as u64))?;
        self.res_type.assign(ctx, F::from(result_type as u64))?;

        self.lookup_stack_read.assign(
            ctx,
            &MemoryTableLookupEncode::encode_stack_read(
                BigUint::from(step_info.current.eid),
                BigUint::from(1u64),
                BigUint::from(step_info.current.sp + 1),
                BigUint::from(value_type as u16),
                BigUint::from(value),
            ),
        )?;

        self.lookup_stack_write.assign(
            ctx,
            &MemoryTableLookupEncode::encode_stack_write(
                BigUint::from(step_info.current.eid),
                BigUint::from(2u64),
                BigUint::from(step_info.current.sp + 1),
                BigUint::from(result_type as u16),
                BigUint::from(result),
            ),
        )?;

        Ok(())
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
                self.value_type.expr(meta),
                self.value.expr(meta),
            )),

            MLookupItem::Second => Some(MemoryTableLookupEncode::encode_stack_write(
                common_config.eid(meta),
                constant_from!(2),
                common_config.sp(meta) + constant_from!(1),
                self.res_type.expr(meta),
                self.res.expr(meta),
            )),

            _ => None,
        }
    }
}
