use crate::{
    circuits::{
        cell::*,
        etable_v2::{
            allocator::*, ConstraintBuilder, EventTableCommonConfig, EventTableOpcodeConfig,
            EventTableOpcodeConfigBuilder,
        },
        utils::{
            bn_to_field, step_status::StepStatus, table_entry::EventTableEntryWithMemoryInfo,
            Context,
        },
    },
    constant, constant_from, constant_from_bn,
};
use halo2_proofs::{
    arithmetic::FieldExt,
    plonk::{Error, Expression, VirtualCells},
};
use num_bigint::BigUint;
use specs::{
    etable::EventTableEntry,
    itable::{BinOp, OpcodeClass, OPCODE_ARG0_SHIFT, OPCODE_ARG1_SHIFT, OPCODE_CLASS_SHIFT},
    mtable::{LocationType, VarType},
    step::StepInfo,
};

pub struct BinConfig<F: FieldExt> {
    lhs: AllocatedU64CellWithFlagBitDyn<F>,
    rhs: AllocatedU64CellWithFlagBitDyn<F>,

    is_i32: AllocatedBitCell<F>,

    d: AllocatedU64Cell<F>,
    d_flag_helper_diff: AllocatedCommonRangeCell<F>,

    res: AllocatedU64Cell<F>,
    aux1: AllocatedU64Cell<F>,
    aux2: AllocatedU64Cell<F>,
    aux3: AllocatedU64Cell<F>,

    overflow: AllocatedBitCell<F>,
    is_add: AllocatedBitCell<F>,
    is_sub: AllocatedBitCell<F>,
    is_mul: AllocatedBitCell<F>,
    is_div_u: AllocatedBitCell<F>,
    is_rem_u: AllocatedBitCell<F>,
    is_div_s: AllocatedBitCell<F>,
    is_rem_s: AllocatedBitCell<F>,

    size_modulus: AllocatedUnlimitedCell<F>,

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

        let res = allocator.alloc_u64_cell();
        let aux1 = allocator.alloc_u64_cell();
        let aux2 = allocator.alloc_u64_cell();
        let aux3 = allocator.alloc_u64_cell();

        let overflow = allocator.alloc_bit_cell();
        let is_add = allocator.alloc_bit_cell();
        let is_sub = allocator.alloc_bit_cell();
        let is_mul = allocator.alloc_bit_cell();
        let is_div_u = allocator.alloc_bit_cell();
        let is_div_s = allocator.alloc_bit_cell();
        let is_rem_u = allocator.alloc_bit_cell();
        let is_rem_s = allocator.alloc_bit_cell();

        let size_modulus = allocator.alloc_unlimited_cell();

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
            "bin: add/sub constraints",
            Box::new(move |meta| {
                // The range of res can be limited with is_i32 in memory table
                vec![
                    (lhs.u64_cell.expr(meta) + rhs.u64_cell.expr(meta)
                        - res.u64_cell.expr(meta)
                        - overflow.expr(meta) * size_modulus.expr(meta))
                        * is_add.expr(meta),
                    (rhs.u64_cell.expr(meta) + res.u64_cell.expr(meta)
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
                        - res.u64_cell.expr(meta))
                        * is_mul.expr(meta),
                ]
            }),
        );

        constraint_builder.push(
            "bin: div_u/rem_u constraints",
            Box::new(move |meta| {
                vec![
                    (lhs.u64_cell.expr(meta)
                        - rhs.u64_cell.expr(meta) * aux1.u64_cell.expr(meta)
                        - aux2.u64_cell.expr(meta))
                        * (is_rem_u.expr(meta) + is_div_u.expr(meta)),
                    (aux2.u64_cell.expr(meta) + aux3.u64_cell.expr(meta) + constant_from!(1)
                        - rhs.u64_cell.expr(meta))
                        * (is_rem_u.expr(meta) + is_div_u.expr(meta)),
                    (res.u64_cell.expr(meta) - aux1.u64_cell.expr(meta)) * is_div_u.expr(meta),
                    (res.u64_cell.expr(meta) - aux2.u64_cell.expr(meta)) * is_rem_u.expr(meta),
                ]
            }),
        );

        constraint_builder.push(
            "bin: div_s/rem_s constraints common",
            Box::new(move |meta| {
                let enable = is_div_s.expr(meta) + is_rem_s.expr(meta);

                let normalized_lhs = lhs.u64_cell.expr(meta)
                    * (constant_from!(1) - lhs.flag_bit_cell.expr(meta))
                    + (size_modulus.expr(meta) - lhs.u64_cell.expr(meta))
                        * lhs.flag_bit_cell.expr(meta);
                let normalized_rhs = rhs.u64_cell.expr(meta)
                    * (constant_from!(1) - rhs.flag_bit_cell.expr(meta))
                    + (size_modulus.expr(meta) - rhs.u64_cell.expr(meta))
                        * rhs.flag_bit_cell.expr(meta);

                let res_flag = lhs.flag_bit_cell.expr(meta) + rhs.flag_bit_cell.expr(meta)
                    - constant_from!(2)
                        * lhs.flag_bit_cell.expr(meta)
                        * rhs.flag_bit_cell.expr(meta);

                let d_leading_u16 = d.u16_cells_le[3].expr(meta)
                    + is_i32.expr(meta)
                        * (d.u16_cells_le[1].expr(meta) - d.u16_cells_le[3].expr(meta));
                vec![
                    // d_flag must be zero if res_flag is zero
                    (d_leading_u16 + d_flag_helper_diff.expr(meta) - constant_from!(0x7fff))
                        * (constant_from!(1) - res_flag.clone()),
                    normalized_lhs
                        - normalized_rhs.clone() * d.u64_cell.expr(meta)
                        - aux1.u64_cell.expr(meta),
                    aux1.u64_cell.expr(meta) + aux2.u64_cell.expr(meta) + constant_from!(1)
                        - normalized_rhs,
                ]
                .into_iter()
                .map(|x| x * enable.clone())
                .collect()
            }),
        );

        constraint_builder.push(
            "bin: div_s constraints res",
            Box::new(move |meta| {
                let res_flag = lhs.flag_bit_cell.expr(meta) + rhs.flag_bit_cell.expr(meta)
                    - constant_from!(2)
                        * lhs.flag_bit_cell.expr(meta)
                        * rhs.flag_bit_cell.expr(meta);

                vec![
                    (res.u64_cell.expr(meta) - d.u64_cell.expr(meta))
                        * (constant_from!(1) - res_flag.clone())
                        * is_div_s.expr(meta),
                    (res.u64_cell.expr(meta) + d.u64_cell.expr(meta) - size_modulus.expr(meta))
                        * (d.u64_cell.expr(meta) + res.u64_cell.expr(meta))
                        * res_flag.clone()
                        * is_div_s.expr(meta),
                ]
            }),
        );

        constraint_builder.push(
            "bin: rem_s constraints res",
            Box::new(move |meta| {
                vec![
                    (res.u64_cell.expr(meta) - aux1.u64_cell.expr(meta))
                        * (constant_from!(1) - lhs.flag_bit_cell.expr(meta))
                        * is_rem_s.expr(meta),
                    (res.u64_cell.expr(meta) + aux1.u64_cell.expr(meta) - size_modulus.expr(meta))
                        * (aux1.u64_cell.expr(meta) + res.u64_cell.expr(meta))
                        * lhs.flag_bit_cell.expr(meta)
                        * is_rem_s.expr(meta),
                ]
            }),
        );

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

        let memory_table_lookup_stack_write = allocator.alloc_memory_table_lookup_write_cell(
            "op_bin stack read",
            constraint_builder,
            eid,
            move |____| constant_from!(LocationType::Stack as u64),
            move |meta| sp.expr(meta) + constant_from!(2),
            move |meta| is_i32.expr(meta),
            move |meta| res.u64_cell.expr(meta),
            move |____| constant_from!(1),
        );

        Box::new(BinConfig {
            lhs,
            rhs,
            is_i32,
            d,
            d_flag_helper_diff,
            res,
            aux1,
            aux2,
            aux3,
            overflow,
            is_add,
            is_sub,
            is_mul,
            is_div_u,
            is_rem_u,
            is_div_s,
            is_rem_s,
            memory_table_lookup_stack_read_lhs,
            memory_table_lookup_stack_read_rhs,
            memory_table_lookup_stack_write,
            size_modulus,
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
        step: &StepStatus,
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

        self.lhs
            .assign(ctx, left.into(), var_type == VarType::I32)?;
        self.rhs
            .assign(ctx, right.into(), var_type == VarType::I32)?;
        self.res.assign(ctx, value.into())?;
        self.size_modulus
            .assign_bn(ctx, &(BigUint::from(1u64) << shift))?;

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
            }
            BinOp::SignedRem => {
                self.is_rem_s.assign(ctx, F::one())?;
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

        match var_type {
            VarType::I32 => {
                self.is_i32.assign(ctx, F::one())?;
            }
            _ => {}
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
