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
use specs::{
    etable::EventTableEntry,
    itable::{OpcodeClass, UnaryOp, OPCODE_ARG0_SHIFT, OPCODE_ARG1_SHIFT, OPCODE_CLASS_SHIFT},
    mtable::VarType,
};

pub struct UnaryConfig {
    operand: U64Cell,
    result: U64Cell,
    operand_inv: UnlimitedCell,
    operand_is_zero: BitCell,

    is_ctz: BitCell,
    is_clz: BitCell,
    is_popcnt: BitCell,
    is_64bits: BitCell,

    boundary: UnlimitedCell,
    aux1: U64OnU8Cell,
    aux2: U64OnU8Cell,

    lookup_pow: PowTableLookupCell,
    lookup_stack_read: MTableLookupCell,
    lookup_stack_write: MTableLookupCell,
}

pub struct UnaryConfigBuilder {}

impl<F: FieldExt> EventTableOpcodeConfigBuilder<F> for UnaryConfigBuilder {
    fn configure(
        common: &mut EventTableCellAllocator<F>,
        constraint_builder: &mut ConstraintBuilder<F>,
    ) -> Box<dyn EventTableOpcodeConfig<F>> {
        let operand = common.alloc_u64();
        let result = common.alloc_u64();
        let operand_is_zero = common.alloc_bit_value();
        let operand_inv = common.alloc_unlimited_value();

        let boundary = common.alloc_unlimited_value();
        let aux1 = common.alloc_u64_on_u8();
        let aux2 = common.alloc_u64_on_u8();

        let is_ctz = common.alloc_bit_value();
        let is_clz = common.alloc_bit_value();
        let is_popcnt = common.alloc_bit_value();
        let is_64bits = common.alloc_bit_value();

        let lookup_pow = common.alloc_pow_table_lookup();
        let lookup_stack_read = common.alloc_mtable_lookup();
        let lookup_stack_write = common.alloc_mtable_lookup();

        constraint_builder.push(
            "op_unary: selector",
            Box::new(move |meta| {
                vec![
                    (is_ctz.expr(meta) + is_clz.expr(meta) + is_popcnt.expr(meta)
                        - constant_from!(1)),
                ]
            }),
        );

        constraint_builder.push(
            "op_unary: zero cond",
            Box::new(move |meta| {
                vec![
                    operand_is_zero.expr(meta) * operand.expr(meta),
                    operand.expr(meta) * operand_inv.expr(meta) - constant_from!(1)
                        + operand_is_zero.expr(meta),
                ]
            }),
        );

        constraint_builder.push(
            "op_unary: clz",
            Box::new(move |meta| {
                let bits = constant_from!(32) + constant_from!(32) * is_64bits.expr(meta);

                let operand_is_not_zero = constant_from!(1) - operand_is_zero.expr(meta);

                vec![
                    operand_is_zero.expr(meta)
                        * (result.expr(meta)
                            - (constant_from!(32) + is_64bits.expr(meta) * constant_from!(32))),
                    operand_is_not_zero.clone()
                        * (boundary.expr(meta) + aux1.expr(meta) - operand.expr(meta)),
                    operand_is_not_zero.clone()
                        * (aux1.expr(meta) + aux2.expr(meta) + constant_from!(1)
                            - boundary.expr(meta)),
                    operand_is_not_zero
                        * (lookup_pow.expr(meta)
                            - pow_table_encode(
                                boundary.expr(meta),
                                bits - result.expr(meta) - constant_from!(1),
                            )),
                ]
                .into_iter()
                .map(|constraint| constraint * is_clz.expr(meta))
                .collect()
            }),
        );

        constraint_builder.push(
            "op_unary: ctz",
            Box::new(move |meta| {
                let operand_is_not_zero = constant_from!(1) - operand_is_zero.expr(meta);

                vec![
                    operand_is_zero.expr(meta)
                        * (result.expr(meta)
                            - (constant_from!(32) + is_64bits.expr(meta) * constant_from!(32))),
                    operand_is_not_zero
                        * (aux1.expr(meta) * boundary.expr(meta) * constant_from!(2)
                            + boundary.expr(meta)
                            - operand.expr(meta)),
                    lookup_pow.expr(meta)
                        - pow_table_encode(boundary.expr(meta), result.expr(meta)),
                ]
                .into_iter()
                .map(|constraint| constraint * is_ctz.expr(meta))
                .collect()
            }),
        );

        // TODO: support popcnt

        Box::new(UnaryConfig {
            operand,
            result,
            operand_inv,
            operand_is_zero,
            is_ctz,
            is_clz,
            is_popcnt,
            is_64bits,
            boundary,
            aux1,
            aux2,
            lookup_pow,
            lookup_stack_read,
            lookup_stack_write,
        })
    }
}

impl<F: FieldExt> EventTableOpcodeConfig<F> for UnaryConfig {
    fn opcode(&self, meta: &mut VirtualCells<'_, F>) -> Expression<F> {
        macro_rules! op_expr {
            ($op: expr, $field: ident) => {
                self.$field.expr(meta)
                    * constant!(bn_to_field(
                        &(BigUint::from($op as u64) << OPCODE_ARG0_SHIFT)
                    ))
            };
        }

        let opcode_class = constant!(bn_to_field(
            &(BigUint::from(OpcodeClass::Unary as u64) << OPCODE_CLASS_SHIFT)
        ));
        let vtype = (self.is_64bits.expr(meta) + constant_from!(1))
            * constant!(bn_to_field(&(BigUint::from(1u64) << OPCODE_ARG1_SHIFT)));
        let op = op_expr!(UnaryOp::Ctz, is_ctz)
            + op_expr!(UnaryOp::Clz, is_clz)
            + op_expr!(UnaryOp::Popcnt, is_popcnt);

        opcode_class + vtype + op
    }

    fn assign(
        &self,
        ctx: &mut Context<'_, F>,
        step_info: &StepStatus,
        entry: &EventTableEntry,
    ) -> Result<(), Error> {
        match &entry.step_info {
            specs::step::StepInfo::UnaryOp {
                class,
                vtype,
                operand,
                result,
            } => {
                self.operand.assign(ctx, *operand)?;
                self.result.assign(ctx, *result)?;
                self.is_64bits.assign(ctx, *vtype == VarType::I64)?;

                self.operand_inv
                    .assign(ctx, F::from(*operand).invert().unwrap_or(F::zero()))?;
                self.operand_is_zero.assign(ctx, *operand == 0)?;

                let (bits, max) = if *vtype == VarType::I32 {
                    (32, 1u128 << 32)
                } else {
                    (64, 1u128 << 64)
                };

                match class {
                    UnaryOp::Ctz => {
                        self.is_ctz.assign(ctx, true)?;

                        /*
                         * 0000 0100 0000 1000
                         * |____________| |__|
                         *  hd            boundary
                         *
                         */
                        let least_one_pos = *result;
                        let hd = (*operand)
                            .checked_shr(least_one_pos as u32 + 1)
                            .unwrap_or(0);

                        self.aux1.assign(ctx, hd)?;
                        self.boundary
                            .assign(ctx, bn_to_field(&BigUint::from(1u128 << least_one_pos)))?;
                        self.lookup_pow.assign(ctx, least_one_pos)?;
                    }
                    UnaryOp::Clz => {
                        self.is_clz.assign(ctx, true)?;

                        /*
                         * operand:
                         *   0000 0100 0000 1000
                         * aux1: tail of operand
                         *    i.e.  00 0000 1000
                         * boundary: operand minus tail
                         *    i.e. 100 0000 0000
                         */
                        let boundary = max.checked_shr(1 + *result as u32).unwrap_or(0) as u64;
                        let tail = *operand ^ boundary;

                        self.boundary.assign(ctx, F::from(boundary as u64))?;
                        self.aux1.assign(ctx, tail)?;
                        self.aux2.assign(ctx, boundary - tail - 1)?;
                        if boundary != 0 {
                            self.lookup_pow.assign(ctx, bits - *result - 1)?;
                        }
                    }
                    UnaryOp::Popcnt => {
                        self.is_popcnt.assign(ctx, true)?;

                        todo!()
                    }
                }

                self.lookup_stack_read.assign(
                    ctx,
                    &MemoryTableLookupEncode::encode_stack_read(
                        BigUint::from(step_info.current.eid),
                        BigUint::from(1 as u64),
                        BigUint::from(step_info.current.sp + 1),
                        BigUint::from(*vtype as u16),
                        BigUint::from(*operand),
                    ),
                )?;

                self.lookup_stack_write.assign(
                    ctx,
                    &MemoryTableLookupEncode::encode_stack_write(
                        BigUint::from(step_info.current.eid),
                        BigUint::from(2 as u64),
                        BigUint::from(step_info.current.sp + 1),
                        BigUint::from(*vtype as u16),
                        BigUint::from(*result),
                    ),
                )?;
            }
            _ => unreachable!(),
        }

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
        let vtype = self.is_64bits.expr(meta) + constant_from!(1);
        match item {
            MLookupItem::First => Some(MemoryTableLookupEncode::encode_stack_read(
                common_config.eid(meta),
                constant_from!(1),
                common_config.sp(meta) + constant_from!(1),
                vtype.clone(),
                self.operand.expr(meta),
            )),
            MLookupItem::Second => Some(MemoryTableLookupEncode::encode_stack_write(
                common_config.eid(meta),
                constant_from!(2),
                common_config.sp(meta) + constant_from!(1),
                vtype.clone(),
                self.result.expr(meta),
            )),
            _ => None,
        }
    }
}
