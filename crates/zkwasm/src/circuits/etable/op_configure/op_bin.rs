use crate::circuits::cell::*;
use crate::circuits::etable::allocator::*;
use crate::circuits::etable::ConstraintBuilder;
use crate::circuits::etable::EventTableCommonConfig;
use crate::circuits::etable::EventTableOpcodeConfig;
use crate::circuits::etable::EventTableOpcodeConfigBuilder;
use crate::circuits::utils::bn_to_field;
use crate::circuits::utils::step_status::StepStatus;
use crate::circuits::utils::table_entry::EventTableEntryWithMemoryInfo;
use crate::circuits::utils::Context;
use crate::constant;
use crate::constant_from;
use crate::constant_from_bn;
use halo2_proofs::arithmetic::FieldExt;
use halo2_proofs::plonk::Error;
use halo2_proofs::plonk::Expression;
use halo2_proofs::plonk::VirtualCells;
use num_bigint::BigUint;
use specs::etable::EventTableEntry;
use specs::itable::BinOp;
use specs::itable::OpcodeClass;
use specs::itable::OPCODE_ARG0_SHIFT;
use specs::itable::OPCODE_ARG1_SHIFT;
use specs::itable::OPCODE_CLASS_SHIFT;
use specs::mtable::LocationType;
use specs::mtable::VarType;
use specs::step::StepInfo;

pub struct BinConfig<F: FieldExt> {
    lhs: AllocatedU64CellWithFlagBitDyn<F>,
    rhs: AllocatedU64CellWithFlagBitDyn<F>,

    is_i32: AllocatedBitCell<F>,

    d: AllocatedU64Cell<F>,
    d_flag_helper_diff: AllocatedCommonRangeCell<F>,

    aux1: AllocatedU64Cell<F>,
    aux2: AllocatedU64Cell<F>,

    overflow: AllocatedBitCell<F>,
    is_add: AllocatedBitCell<F>,
    is_sub: AllocatedBitCell<F>,
    is_mul: AllocatedBitCell<F>,
    is_div_u: AllocatedBitCell<F>,
    is_rem_u: AllocatedBitCell<F>,
    is_div_s: AllocatedBitCell<F>,
    is_rem_s: AllocatedBitCell<F>,
    is_div_s_or_rem_s: AllocatedBitCell<F>,

    res_flag: AllocatedUnlimitedCell<F>,
    size_modulus: AllocatedUnlimitedCell<F>,
    normalized_lhs: AllocatedUnlimitedCell<F>,
    normalized_rhs: AllocatedUnlimitedCell<F>,
    d_leading_u16: AllocatedUnlimitedCell<F>,
    degree_helper1: AllocatedUnlimitedCell<F>,
    degree_helper2: AllocatedUnlimitedCell<F>,

    memory_table_lookup_stack_read_lhs: AllocatedMemoryTableLookupReadCell<F>,
    memory_table_lookup_stack_read_rhs: AllocatedMemoryTableLookupReadCell<F>,
    memory_table_lookup_stack_write: AllocatedMemoryTableLookupWriteCell<F>,
}

pub struct BinConfigBuilder {}

impl<F: FieldExt> EventTableOpcodeConfigBuilder<F> for BinConfigBuilder {
    fn configure(
        common_config: &EventTableCommonConfig<F>,
        allocator: &mut EventTableCellAllocator<F>,
        constraint_builder: &mut ConstraintBuilder<F>,
    ) -> Box<dyn EventTableOpcodeConfig<F>> {
        let is_i32 = allocator.alloc_bit_cell();
        let lhs = allocator
            .alloc_u64_with_flag_bit_cell_dyn(constraint_builder, move |meta| is_i32.expr(meta));
        let rhs = allocator
            .alloc_u64_with_flag_bit_cell_dyn(constraint_builder, move |meta| is_i32.expr(meta));

        let d = allocator.alloc_u64_cell();
        let d_flag_helper_diff = allocator.alloc_common_range_cell();

        let aux1 = allocator.alloc_u64_cell();
        let aux2 = allocator.alloc_u64_cell();

        let overflow = allocator.alloc_bit_cell();
        let is_add = allocator.alloc_bit_cell();
        let is_sub = allocator.alloc_bit_cell();
        let is_mul = allocator.alloc_bit_cell();
        let is_div_u = allocator.alloc_bit_cell();
        let is_div_s = allocator.alloc_bit_cell();
        let is_rem_u = allocator.alloc_bit_cell();
        let is_rem_s = allocator.alloc_bit_cell();

        let is_div_s_or_rem_s = allocator.alloc_bit_cell();

        let d_leading_u16 = allocator.alloc_unlimited_cell();
        let normalized_lhs = allocator.alloc_unlimited_cell();
        let normalized_rhs = allocator.alloc_unlimited_cell();
        let res_flag = allocator.alloc_unlimited_cell();
        let size_modulus = allocator.alloc_unlimited_cell();
        let degree_helper1 = allocator.alloc_unlimited_cell();
        let degree_helper2 = allocator.alloc_unlimited_cell();

        let eid = common_config.eid_cell;
        let sp = common_config.sp_cell;

        let memory_table_lookup_stack_read_rhs = allocator.alloc_memory_table_lookup_read_cell(
            "op_bin stack read",
            constraint_builder,
            eid,
            move |____| constant_from!(LocationType::Stack as u64),
            move |meta| sp.expr(meta) + constant_from!(1),
            move |meta| is_i32.expr(meta),
            move |meta| rhs.u64_cell.expr(meta),
            move |____| constant_from!(1),
        );

        let memory_table_lookup_stack_read_lhs = allocator.alloc_memory_table_lookup_read_cell(
            "op_bin stack read",
            constraint_builder,
            eid,
            move |____| constant_from!(LocationType::Stack as u64),
            move |meta| sp.expr(meta) + constant_from!(2),
            move |meta| is_i32.expr(meta),
            move |meta| lhs.u64_cell.expr(meta),
            move |____| constant_from!(1),
        );

        let memory_table_lookup_stack_write = allocator
            .alloc_memory_table_lookup_write_cell_with_value(
                "op_bin stack read",
                constraint_builder,
                eid,
                move |____| constant_from!(LocationType::Stack as u64),
                move |meta| sp.expr(meta) + constant_from!(2),
                move |meta| is_i32.expr(meta),
                move |____| constant_from!(1),
            );

        let res = memory_table_lookup_stack_write.value_cell;

        constraint_builder.push(
            "bin: selector",
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

        // cs: size_modulus = if is_i32 { 1 << 32 } else { 1 << 64 }
        constraint_builder.push(
            "bin: size modulus",
            Box::new(move |meta| {
                vec![
                    size_modulus.expr(meta) - constant_from_bn!(&(BigUint::from(1u64) << 64usize))
                        + is_i32.expr(meta) * constant_from!((u32::MAX as u64) << 32),
                ]
            }),
        );

        constraint_builder.push(
            "c.bin.add",
            Box::new(move |meta| {
                // The range of res can be limited with is_i32 in memory table
                vec![
                    (lhs.u64_cell.expr(meta) + rhs.u64_cell.expr(meta)
                        - res.expr(meta)
                        - overflow.expr(meta) * size_modulus.expr(meta))
                        * is_add.expr(meta),
                ]
            }),
        );

        constraint_builder.push(
            "c.bin.sub",
            Box::new(move |meta| {
                // The range of res can be limited with is_i32 in memory table
                vec![
                    (rhs.u64_cell.expr(meta) + res.expr(meta)
                        - lhs.u64_cell.expr(meta)
                        - overflow.expr(meta) * size_modulus.expr(meta))
                        * is_sub.expr(meta),
                ]
            }),
        );

        constraint_builder.push(
            "bin: mul constraints",
            Box::new(move |meta| {
                // The range of res can be limited with is_i32 in memory table
                vec![
                    (lhs.u64_cell.expr(meta) * rhs.u64_cell.expr(meta)
                        - aux1.u64_cell.expr(meta) * size_modulus.expr(meta)
                        - res.expr(meta))
                        * is_mul.expr(meta),
                ]
            }),
        );

        constraint_builder.push(
            "bin: div_u/rem_u constraints",
            Box::new(move |meta| {
                vec![
                    (lhs.u64_cell.expr(meta)
                        - rhs.u64_cell.expr(meta) * d.u64_cell.expr(meta)
                        - aux1.u64_cell.expr(meta))
                        * (is_rem_u.expr(meta) + is_div_u.expr(meta)),
                    (aux1.u64_cell.expr(meta) + aux2.u64_cell.expr(meta) + constant_from!(1)
                        - rhs.u64_cell.expr(meta))
                        * (is_rem_u.expr(meta) + is_div_u.expr(meta)),
                    (res.expr(meta) - d.u64_cell.expr(meta)) * is_div_u.expr(meta),
                    (res.expr(meta) - aux1.u64_cell.expr(meta)) * is_rem_u.expr(meta),
                ]
            }),
        );

        constraint_builder.push(
            "bin: res flag",
            Box::new(move |meta| {
                vec![
                    res_flag.expr(meta)
                        - (lhs.flag_bit_cell.expr(meta) + rhs.flag_bit_cell.expr(meta)
                            - constant_from!(2)
                                * lhs.flag_bit_cell.expr(meta)
                                * rhs.flag_bit_cell.expr(meta)),
                ]
            }),
        );

        constraint_builder.push(
            "bin: div_s/rem_s constraints common",
            Box::new(move |meta| {
                let normalized_lhs_expr = lhs.u64_cell.expr(meta)
                    * (constant_from!(1) - lhs.flag_bit_cell.expr(meta))
                    + (size_modulus.expr(meta) - lhs.u64_cell.expr(meta))
                        * lhs.flag_bit_cell.expr(meta);
                let normalized_rhs_expr = rhs.u64_cell.expr(meta)
                    * (constant_from!(1) - rhs.flag_bit_cell.expr(meta))
                    + (size_modulus.expr(meta) - rhs.u64_cell.expr(meta))
                        * rhs.flag_bit_cell.expr(meta);

                let d_leading_u16_expr = d.u16_cells_le[3].expr(meta)
                    + is_i32.expr(meta)
                        * (d.u16_cells_le[1].expr(meta) - d.u16_cells_le[3].expr(meta));
                vec![
                    // d_flag must be zero if res_flag is zero
                    is_div_s_or_rem_s.expr(meta) - (is_div_s.expr(meta) + is_rem_s.expr(meta)),
                    normalized_lhs.expr(meta) - normalized_lhs_expr,
                    normalized_rhs.expr(meta) - normalized_rhs_expr,
                    (d_leading_u16.expr(meta) - d_leading_u16_expr) * is_div_s_or_rem_s.expr(meta),
                    (d_leading_u16.expr(meta) + d_flag_helper_diff.expr(meta)
                        - constant_from!(0x7fff))
                        * (constant_from!(1) - res_flag.expr(meta))
                        * is_div_s_or_rem_s.expr(meta),
                    (normalized_lhs.expr(meta)
                        - normalized_rhs.expr(meta) * d.u64_cell.expr(meta)
                        - aux1.u64_cell.expr(meta))
                        * is_div_s_or_rem_s.expr(meta),
                    (aux1.u64_cell.expr(meta) + aux2.u64_cell.expr(meta) + constant_from!(1)
                        - normalized_rhs.expr(meta))
                        * is_div_s_or_rem_s.expr(meta),
                ]
            }),
        );

        constraint_builder.push(
            "bin: div_s constraints res",
            Box::new(move |meta| {
                vec![
                    (res.expr(meta) - d.u64_cell.expr(meta))
                        * (constant_from!(1) - res_flag.expr(meta))
                        * is_div_s.expr(meta),
                    (degree_helper1.expr(meta)
                        - (d.u64_cell.expr(meta) + res.expr(meta)) * res_flag.expr(meta))
                        * is_div_s.expr(meta),
                    /*
                     * If only one of the left and the right is negative,
                     * `res` must equal to `size_modulus - normalized quotient(d)`, or
                     * `res` and `d` are both zero.
                     */
                    (res.expr(meta) + d.u64_cell.expr(meta) - size_modulus.expr(meta))
                        * degree_helper1.expr(meta)
                        * is_div_s.expr(meta),
                ]
            }),
        );

        constraint_builder.push(
            "bin: rem_s constraints res",
            Box::new(move |meta| {
                vec![
                    (res.expr(meta) - aux1.u64_cell.expr(meta))
                        * (constant_from!(1) - lhs.flag_bit_cell.expr(meta))
                        * is_rem_s.expr(meta),
                    (degree_helper2.expr(meta)
                        - (aux1.u64_cell.expr(meta) + res.expr(meta))
                            * lhs.flag_bit_cell.expr(meta)) // The sign of the left operator determines the flag bit of the result value.
                        * is_rem_s.expr(meta),
                    (res.expr(meta) + aux1.u64_cell.expr(meta) - size_modulus.expr(meta))
                        * degree_helper2.expr(meta)
                        * is_rem_s.expr(meta),
                ]
            }),
        );

        Box::new(BinConfig {
            lhs,
            rhs,
            is_i32,
            d,
            d_flag_helper_diff,
            aux1,
            aux2,
            overflow,
            is_add,
            is_sub,
            is_mul,
            is_div_u,
            is_rem_u,
            is_div_s,
            is_rem_s,
            is_div_s_or_rem_s,
            memory_table_lookup_stack_read_lhs,
            memory_table_lookup_stack_read_rhs,
            memory_table_lookup_stack_write,
            size_modulus,
            res_flag,
            normalized_lhs,
            normalized_rhs,
            d_leading_u16,
            degree_helper1,
            degree_helper2,
        })
    }
}

impl<F: FieldExt> EventTableOpcodeConfig<F> for BinConfig<F> {
    fn opcode(&self, meta: &mut VirtualCells<'_, F>) -> Expression<F> {
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
            + self.is_i32.expr(meta)
                * constant!(bn_to_field(&(BigUint::from(1u64) << OPCODE_ARG1_SHIFT)))
    }

    fn assign(
        &self,
        ctx: &mut Context<'_, F>,
        step: &mut StepStatus<F>,
        entry: &EventTableEntryWithMemoryInfo,
    ) -> Result<(), Error> {
        let (class, var_type, shift, left, right, value) = match &entry.eentry.step_info {
            StepInfo::I32BinOp {
                class,
                left,
                right,
                value,
            } => {
                let var_type = VarType::I32;
                let left = *left as u32 as u64;
                let right = *right as u32 as u64;
                let value = *value as u32 as u64;

                (class, var_type, 32, left, right, value)
            }

            StepInfo::I64BinOp {
                class,
                left,
                right,
                value,
            } => {
                let var_type = VarType::I64;
                let left = *left as u64;
                let right = *right as u64;
                let value = *value as u64;

                (class, var_type, 64, left, right, value)
            }

            _ => unreachable!(),
        };

        self.lhs.assign(ctx, left, var_type == VarType::I32)?;
        self.rhs.assign(ctx, right, var_type == VarType::I32)?;

        let (normalized_lhs, normalized_rhs) = if var_type == VarType::I32 {
            let normalized_lhs = if left >> 31 == 1 {
                u32::MAX as u64 - left + 1
            } else {
                left
            };
            let normalized_rhs = if right >> 31 == 1 {
                u32::MAX as u64 - right + 1
            } else {
                right
            };
            (normalized_lhs, normalized_rhs)
        } else {
            let normalized_lhs = if left >> 63 == 1 {
                u64::MAX - left + 1
            } else {
                left
            };
            let normalized_rhs = if right >> 63 == 1 {
                u64::MAX - right + 1
            } else {
                right
            };
            (normalized_lhs, normalized_rhs)
        };
        self.normalized_lhs.assign(ctx, normalized_lhs.into())?;
        self.normalized_rhs.assign(ctx, normalized_rhs.into())?;

        self.size_modulus
            .assign_bn(ctx, &(BigUint::from(1u64) << shift))?;

        let (lhs_flag, res_flag) = {
            let shift = if var_type == VarType::I32 { 31 } else { 63 };
            let lhs_flag = left >> shift;
            let rhs_flag = right >> shift;
            let res_flag = lhs_flag ^ rhs_flag;
            self.res_flag.assign(ctx, res_flag.into())?;

            (lhs_flag, res_flag)
        };

        match class {
            BinOp::Add => {
                self.is_add.assign(ctx, F::one())?;
                self.overflow.assign_bn(
                    ctx,
                    &((BigUint::from(left) + BigUint::from(right)) >> shift),
                )?;
            }
            BinOp::Sub => {
                self.is_sub.assign(ctx, F::one())?;
                self.overflow.assign_bn(
                    ctx,
                    &((BigUint::from(right) + BigUint::from(value)) >> shift),
                )?;
            }
            BinOp::Mul => {
                self.is_mul.assign(ctx, F::one())?;
                self.aux1
                    .assign(ctx, ((left as u128 * right as u128) >> shift) as u64)?;
            }
            BinOp::UnsignedDiv => {
                self.is_div_u.assign(ctx, F::one())?;
            }
            BinOp::UnsignedRem => {
                self.is_rem_u.assign(ctx, F::one())?;
            }
            BinOp::SignedDiv => {
                self.is_div_s.assign(ctx, F::one())?;
                self.is_div_s_or_rem_s.assign(ctx, F::one())?;
            }
            BinOp::SignedRem => {
                self.is_rem_s.assign(ctx, F::one())?;
                self.is_div_s_or_rem_s.assign(ctx, F::one())?;
            }
        };

        match class {
            BinOp::UnsignedDiv | BinOp::UnsignedRem => {
                self.d.assign(ctx, left / right)?;
                self.aux1.assign(ctx, left % right)?;
                self.aux2.assign(ctx, right - left % right - 1)?;
            }
            BinOp::SignedDiv | BinOp::SignedRem => {
                let left_flag = left >> (shift - 1) != 0;
                let right_flag = right >> (shift - 1) != 0;

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
                let d_leading_u16 = d >> (shift - 16);

                self.degree_helper1
                    .assign(ctx, (F::from(d) + F::from(value)) * F::from(res_flag))?;

                self.degree_helper2.assign_bn(
                    ctx,
                    &((BigUint::from(rem) + BigUint::from(value)) * lhs_flag),
                )?;
                self.d_leading_u16.assign(ctx, d_leading_u16.into())?;
                self.d_flag_helper_diff.assign(
                    ctx,
                    if d_leading_u16 >= 0x7fff {
                        F::from(0)
                    } else {
                        F::from(0x7fff - d_leading_u16)
                    },
                )?;
                self.d.assign(ctx, d)?;
                self.aux1.assign(ctx, rem)?;
                self.aux2.assign(ctx, normalized_rhs - rem - 1)?;
            }
            _ => {}
        }

        if var_type == VarType::I32 {
            self.is_i32.assign(ctx, F::one())?;
        };

        self.memory_table_lookup_stack_read_rhs.assign(
            ctx,
            entry.memory_rw_entires[0].start_eid,
            step.current.eid,
            entry.memory_rw_entires[0].end_eid,
            step.current.sp + 1,
            LocationType::Stack,
            var_type == VarType::I32,
            right,
        )?;

        self.memory_table_lookup_stack_read_lhs.assign(
            ctx,
            entry.memory_rw_entires[1].start_eid,
            step.current.eid,
            entry.memory_rw_entires[1].end_eid,
            step.current.sp + 2,
            LocationType::Stack,
            var_type == VarType::I32,
            left,
        )?;

        self.memory_table_lookup_stack_write.assign(
            ctx,
            step.current.eid,
            entry.memory_rw_entires[2].end_eid,
            step.current.sp + 2,
            LocationType::Stack,
            var_type == VarType::I32,
            value,
        )?;

        Ok(())
    }

    fn mops(&self, _meta: &mut VirtualCells<'_, F>) -> Option<Expression<F>> {
        Some(constant_from!(1))
    }

    fn memory_writing_ops(&self, _: &EventTableEntry) -> u32 {
        1
    }

    fn sp_diff(&self, _meta: &mut VirtualCells<'_, F>) -> Option<Expression<F>> {
        Some(constant!(F::one()))
    }
}
