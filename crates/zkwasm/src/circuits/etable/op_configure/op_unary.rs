use crate::circuits::bit_table::BitTableOp;
use crate::circuits::cell::*;
use crate::circuits::etable::allocator::*;
use crate::circuits::etable::ConstraintBuilder;
use crate::circuits::etable::EventTableCommonArgsConfig;
use crate::circuits::etable::EventTableCommonConfig;
use crate::circuits::etable::EventTableOpcodeConfig;
use crate::circuits::etable::EventTableOpcodeConfigBuilder;
use crate::circuits::rtable::pow_table_power_encode;
use crate::circuits::utils::bn_to_field;
use crate::circuits::utils::step_status::StepStatus;
use crate::circuits::utils::table_entry::EventTableEntryWithMemoryInfo;
use crate::circuits::utils::Context;
use crate::constant;
use crate::constant_from;
use halo2_proofs::arithmetic::FieldExt;
use halo2_proofs::plonk::Error;
use halo2_proofs::plonk::Expression;
use halo2_proofs::plonk::VirtualCells;
use num_bigint::BigUint;
use specs::encode::opcode::encode_unary;
use specs::encode::opcode::UniArgEncode;
use specs::etable::EventTableEntry;
use specs::itable::UnaryOp;
use specs::mtable::LocationType;
use specs::mtable::VarType;
use specs::step::StepInfo;

pub struct UnaryConfig<F: FieldExt> {
    operand_inv: AllocatedUnlimitedCell<F>,
    bits: AllocatedUnlimitedCell<F>,
    operand_is_zero: AllocatedBitCell<F>,

    is_ctz: AllocatedBitCell<F>,
    is_clz: AllocatedBitCell<F>,
    is_popcnt: AllocatedBitCell<F>,

    aux1: AllocatedU64Cell<F>,
    aux2: AllocatedU64Cell<F>,

    lookup_pow_modulus: AllocatedUnlimitedCell<F>,
    lookup_pow_power: AllocatedUnlimitedCell<F>,
    // To support popcnt
    bit_table_lookup: AllocatedBitTableLookupCells<F>,

    ctz_degree_helper: AllocatedUnlimitedCell<F>,

    operand_arg: EventTableCommonArgsConfig<F>,
    memory_table_lookup_stack_write: AllocatedMemoryTableLookupWriteCell<F>,
}

pub struct UnaryConfigBuilder {}

impl<F: FieldExt> EventTableOpcodeConfigBuilder<F> for UnaryConfigBuilder {
    fn configure<'a>(
        common_config: &'a EventTableCommonConfig<F>,
        allocator: &'a mut EventTableCellAllocator<F>,
        constraint_builder: &'a mut ConstraintBuilder<F>,
    ) -> Box<dyn EventTableOpcodeConfig<F>> {
        let operand_is_zero = allocator.alloc_bit_cell();
        let operand_inv = allocator.alloc_unlimited_cell();
        let bits = allocator.alloc_unlimited_cell();

        let is_ctz = allocator.alloc_bit_cell();
        let is_clz = allocator.alloc_bit_cell();
        let is_popcnt = allocator.alloc_bit_cell();

        let aux1 = allocator.alloc_u64_cell();
        let aux2 = allocator.alloc_u64_cell();

        let ctz_degree_helper = allocator.alloc_unlimited_cell();

        let lookup_pow_modulus = common_config.pow_table_lookup_modulus_cell;
        let lookup_pow_power = common_config.pow_table_lookup_power_cell;
        let lookup_popcnt = common_config.bit_table_lookup_cells;

        let eid = common_config.eid_cell;
        let sp = common_config.sp_cell;

        let operand_arg = common_config.uniarg_configs[0].clone();
        let operand = operand_arg.value_cell;
        let is_i32 = operand_arg.is_i32_cell;

        let uniarg_configs = common_config.uniarg_configs.clone();
        let memory_table_lookup_stack_write = allocator
            .alloc_memory_table_lookup_write_cell_with_value(
                "op_unary stack write",
                constraint_builder,
                eid,
                move |____| constant_from!(LocationType::Stack as u64),
                move |meta| Self::sp_after_uniarg(sp, &uniarg_configs, meta),
                move |meta| is_i32.expr(meta),
                move |____| constant_from!(1),
            );
        let result = memory_table_lookup_stack_write.value_cell;

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
            "op_unary: bits",
            Box::new(move |meta| {
                vec![bits.expr(meta) - constant_from!(64) + constant_from!(32) * is_i32.expr(meta)]
            }),
        );

        constraint_builder.push(
            "op_unary: clz",
            Box::new(move |meta| {
                let operand_is_not_zero = constant_from!(1) - operand_is_zero.expr(meta);

                vec![
                    operand_is_zero.expr(meta) * (result.expr(meta) - bits.expr(meta)),
                    operand_is_not_zero.clone()
                        * (lookup_pow_modulus.expr(meta) + aux1.u64_cell.expr(meta)
                            - operand.expr(meta)),
                    operand_is_not_zero.clone()
                        * (aux1.u64_cell.expr(meta) + aux2.u64_cell.expr(meta) + constant_from!(1)
                            - lookup_pow_modulus.expr(meta)),
                    operand_is_not_zero
                        * (lookup_pow_power.expr(meta)
                            - pow_table_power_encode(
                                bits.expr(meta) - result.expr(meta) - constant_from!(1),
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
                    ctz_degree_helper.expr(meta)
                        - (aux1.u64_cell.expr(meta)
                            * lookup_pow_modulus.expr(meta)
                            * constant_from!(2)),
                    operand_is_zero.expr(meta) * (result.expr(meta) - bits.expr(meta)),
                    operand_is_not_zero
                        * (ctz_degree_helper.expr(meta) + lookup_pow_modulus.expr(meta)
                            - operand.expr(meta)),
                    lookup_pow_power.expr(meta) - pow_table_power_encode(result.expr(meta)),
                ]
                .into_iter()
                .map(|constraint| constraint * is_ctz.expr(meta))
                .collect()
            }),
        );

        constraint_builder.push(
            "op_unary: lookup popcnt",
            Box::new(move |meta| {
                vec![
                    lookup_popcnt.op.expr(meta) - constant_from!(BitTableOp::Popcnt.index()),
                    lookup_popcnt.left.expr(meta) - operand.expr(meta),
                    lookup_popcnt.result.expr(meta) - result.expr(meta),
                ]
                .into_iter()
                .map(|constraint| constraint * is_popcnt.expr(meta))
                .collect()
            }),
        );

        Box::new(UnaryConfig {
            operand_inv,
            bits,
            operand_is_zero,
            is_ctz,
            is_clz,
            is_popcnt,
            aux1,
            aux2,
            lookup_pow_modulus,
            lookup_pow_power,
            ctz_degree_helper,
            bit_table_lookup: lookup_popcnt,
            operand_arg,
            memory_table_lookup_stack_write,
        })
    }
}

impl<F: FieldExt> EventTableOpcodeConfig<F> for UnaryConfig<F> {
    fn opcode(&self, meta: &mut VirtualCells<'_, F>) -> Expression<F> {
        macro_rules! op_expr {
            ($op: expr, $field: ident) => {
                self.$field.expr(meta) * constant!(bn_to_field(&(BigUint::from($op as u64))))
            };
        }

        let op = op_expr!(UnaryOp::Ctz, is_ctz)
            + op_expr!(UnaryOp::Clz, is_clz)
            + op_expr!(UnaryOp::Popcnt, is_popcnt);

        encode_unary(
            op,
            self.operand_arg.is_i32_cell.expr(meta),
            UniArgEncode::Reserve,
        )
    }

    fn assign(
        &self,
        ctx: &mut Context<'_, F>,
        step: &mut StepStatus<F>,
        entry: &EventTableEntryWithMemoryInfo,
    ) -> Result<(), Error> {
        match &entry.eentry.step_info {
            StepInfo::UnaryOp {
                class,
                vtype,
                operand,
                result,
            } => {
                if *operand != 0 {
                    self.operand_inv
                        .assign(ctx, step.field_helper.invert(*operand))?;
                }
                self.operand_is_zero.assign_bool(ctx, *operand == 0)?;

                let (bits, max) = if *vtype == VarType::I32 {
                    (32, 1u128 << 32)
                } else {
                    (64, 1u128 << 64)
                };
                self.bits.assign(ctx, F::from(bits))?;

                match class {
                    UnaryOp::Ctz => {
                        self.is_ctz.assign_bool(ctx, true)?;

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
                        let boundary = bn_to_field(&BigUint::from(1u128 << least_one_pos));

                        self.aux1.assign(ctx, hd)?;
                        self.lookup_pow_modulus.assign(ctx, boundary)?;
                        self.lookup_pow_power.assign(
                            ctx,
                            bn_to_field(&pow_table_power_encode(BigUint::from(least_one_pos))),
                        )?;

                        self.ctz_degree_helper
                            .assign(ctx, F::from(hd) * boundary * F::from(2))?;
                    }
                    UnaryOp::Clz => {
                        self.is_clz.assign_bool(ctx, true)?;

                        /*
                         * operand:
                         *   0000 0100 0000 1000
                         * aux1: tail of operand
                         *    i.e.  00 0000 1000
                         * boundary: operand minus tail
                         *    i.e. 100 0000 0000
                         * aux2: make sure aux1(tail) is less than boundary
                         *    boundary - aux1 - 1
                         */
                        let boundary = max.checked_shr(1 + *result as u32).unwrap_or(0) as u64;
                        let tail = *operand ^ boundary;

                        self.lookup_pow_modulus.assign(ctx, F::from(boundary))?;
                        self.aux1.assign(ctx, tail)?;
                        // If `operand = 0``, then `boundary == tail == 0`` and therefore `- 1` will panic in debug mode.
                        // Since `aux2`` is useless when `operand = 0`, we give 0.
                        let aux2 = (boundary - tail).saturating_sub(1);
                        self.aux2.assign(ctx, aux2)?;
                        if boundary != 0 {
                            self.lookup_pow_modulus.assign(ctx, boundary.into())?;
                            self.lookup_pow_power.assign(
                                ctx,
                                bn_to_field(&pow_table_power_encode(BigUint::from(
                                    bits - *result - 1,
                                ))),
                            )?;
                        }
                    }
                    UnaryOp::Popcnt => {
                        self.is_popcnt.assign_bool(ctx, true)?;

                        self.bit_table_lookup.assign(
                            ctx,
                            BitTableOp::Popcnt,
                            *operand,
                            0,
                            *result,
                        )?;
                    }
                }

                if let specs::itable::Opcode::Unary { uniarg, .. } =
                    entry.eentry.get_instruction(step.current.itable).opcode
                {
                    let mut memory_entries = entry.memory_rw_entires.iter();

                    self.operand_arg.assign(ctx, &uniarg, &mut memory_entries)?;
                } else {
                    unreachable!();
                }

                self.memory_table_lookup_stack_write.assign(
                    ctx,
                    step.current.eid,
                    entry.memory_rw_entires[1].end_eid,
                    step.current.sp + 1,
                    LocationType::Stack,
                    *vtype == VarType::I32,
                    *result as u32 as u64,
                )?;

                Ok(())
            }

            _ => unreachable!(),
        }
    }

    fn mops(&self, _meta: &mut VirtualCells<'_, F>) -> Option<Expression<F>> {
        Some(constant_from!(1))
    }

    fn memory_writing_ops(&self, _: &EventTableEntry) -> u32 {
        1
    }

    fn sp_diff(&self, _meta: &mut VirtualCells<'_, F>) -> Option<Expression<F>> {
        Some(constant!(-F::one()))
    }
}
