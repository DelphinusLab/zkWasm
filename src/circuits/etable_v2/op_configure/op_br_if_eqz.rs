use crate::{
    circuits::{
        cell::*,
        etable_v2::{
            allocator::*, ConstraintBuilder, EventTableCommonConfig, EventTableOpcodeConfig,
            EventTableOpcodeConfigBuilder,
        },
        jtable::{expression::JtableLookupEntryEncode, JumpTableConfig},
        utils::{
            bn_to_field, step_status::StepStatus, table_entry::EventTableEntryWithMemoryInfo,
            Context,
        },
    },
    constant, constant_from,
};
use halo2_proofs::{
    arithmetic::FieldExt,
    plonk::{Error, Expression, VirtualCells},
};
use num_bigint::{BigUint, ToBigUint};
use specs::{
    encode::{frame_table::encode_frame_table_entry, opcode::encode_br, FromBn},
    etable::EventTableEntry,
    itable::{OpcodeClass, OPCODE_ARG0_SHIFT, OPCODE_ARG1_SHIFT, OPCODE_CLASS_SHIFT},
    mtable::{LocationType, VarType},
    step::StepInfo,
};

pub struct BrIfEqzConfig<F: FieldExt> {
    cond_cell: AllocatedU64Cell<F>,
    cond_inv_cell: AllocatedUnlimitedCell<F>,
    cond_is_zero_cell: AllocatedBitCell<F>,
    cond_is_not_zero_cell: AllocatedBitCell<F>,

    keep_cell: AllocatedBitCell<F>,
    is_i32_cell: AllocatedBitCell<F>,
    drop_cell: AllocatedCommonRangeCell<F>,
    dst_pc_cell: AllocatedCommonRangeCell<F>,
    value_cell: AllocatedU64Cell<F>,
    memory_table_lookup_stack_read_cond: AllocatedMemoryTableLookupReadCell<F>,
    memory_table_lookup_stack_read_return_value: AllocatedMemoryTableLookupReadCell<F>,
    memory_table_lookup_stack_write_return_value: AllocatedMemoryTableLookupWriteCell<F>,
}

pub struct BrIfEqzConfigBuilder;

impl<F: FieldExt> EventTableOpcodeConfigBuilder<F> for BrIfEqzConfigBuilder {
    fn configure(
        common_config: &EventTableCommonConfig<F>,
        allocator: &mut EventTableCellAllocator<F>,
        constraint_builder: &mut ConstraintBuilder<F>,
    ) -> Box<dyn EventTableOpcodeConfig<F>> {
        let cond_cell = allocator.alloc_u64_cell();
        let cond_inv_cell = allocator.alloc_unlimited_cell();
        let cond_is_zero_cell = allocator.alloc_bit_cell();
        let cond_is_not_zero_cell = allocator.alloc_bit_cell();

        constraint_builder.constraints.push((
            "op_br_if cond bit",
            Box::new(move |meta| {
                vec![
                    cond_is_zero_cell.expr(meta) * cond_cell.u64_cell.expr(meta),
                    cond_is_zero_cell.expr(meta)
                        + cond_cell.u64_cell.expr(meta) * cond_inv_cell.expr(meta)
                        - constant_from!(1),
                    cond_is_zero_cell.expr(meta) + cond_is_not_zero_cell.expr(meta)
                        - constant_from!(1),
                ]
            }),
        ));

        let keep_cell = allocator.alloc_bit_cell();
        let is_i32_cell = allocator.alloc_bit_cell();
        let drop_cell = allocator.alloc_common_range_cell();
        let dst_pc_cell = allocator.alloc_common_range_cell();
        let value_cell = allocator.alloc_u64_cell();

        let fid_cell = common_config.fid_cell;
        let iid_cell = common_config.iid_cell;
        let frame_id_cell = common_config.frame_id_cell;
        let eid = common_config.eid_cell;
        let sp = common_config.sp_cell;

        let memory_table_lookup_stack_read_cond = allocator.alloc_memory_table_lookup_read_cell(
            "op_br_if stack read cond",
            constraint_builder,
            eid,
            move |meta| constant_from!(LocationType::Stack as u64),
            move |meta| sp.expr(meta) + constant_from!(1),
            move |meta| constant_from!(1),
            move |meta| cond_cell.u64_cell.expr(meta),
            move |meta| constant_from!(1),
        );
        let memory_table_lookup_stack_read_return_value = allocator
            .alloc_memory_table_lookup_read_cell(
                "op_br_if_eqz stack read return value",
                constraint_builder,
                eid,
                move |meta| constant_from!(LocationType::Stack as u64),
                move |meta| sp.expr(meta) + constant_from!(2),
                move |meta| is_i32_cell.expr(meta),
                move |meta| value_cell.u64_cell.expr(meta),
                move |meta| keep_cell.expr(meta) * cond_is_zero_cell.expr(meta),
            );
        let memory_table_lookup_stack_write_return_value = allocator
            .alloc_memory_table_lookup_write_cell(
                "op_br_if_eqz stack write return value",
                constraint_builder,
                eid,
                move |meta| constant_from!(LocationType::Stack as u64),
                move |meta| sp.expr(meta) + drop_cell.expr(meta) + constant_from!(2),
                move |meta| is_i32_cell.expr(meta),
                move |meta| value_cell.u64_cell.expr(meta),
                move |meta| keep_cell.expr(meta) * cond_is_zero_cell.expr(meta),
            );

        Box::new(BrIfEqzConfig {
            cond_cell,
            cond_inv_cell,
            cond_is_zero_cell,
            cond_is_not_zero_cell,
            keep_cell,
            is_i32_cell,
            drop_cell,
            dst_pc_cell,
            value_cell,
            memory_table_lookup_stack_read_cond,
            memory_table_lookup_stack_read_return_value,
            memory_table_lookup_stack_write_return_value,
        })
    }
}

impl<F: FieldExt> EventTableOpcodeConfig<F> for BrIfEqzConfig<F> {
    fn opcode(&self, meta: &mut VirtualCells<'_, F>) -> Expression<F> {
        constant!(bn_to_field(
            &(BigUint::from(OpcodeClass::BrIfEqz as u64) << OPCODE_CLASS_SHIFT)
        )) + self.drop_cell.expr(meta)
            * constant!(bn_to_field(&(BigUint::from(1u64) << OPCODE_ARG0_SHIFT)))
            + self.keep_cell.expr(meta)
                * constant!(bn_to_field(&(BigUint::from(1u64) << OPCODE_ARG1_SHIFT)))
            + self.dst_pc_cell.expr(meta)
    }

    fn assign(
        &self,
        ctx: &mut Context<'_, F>,
        step: &StepStatus,
        entry: &EventTableEntryWithMemoryInfo,
    ) -> Result<(), Error> {
        match &entry.eentry.step_info {
            StepInfo::BrIfEqz {
                condition,
                dst_pc,
                drop,
                keep,
                keep_values,
            } => {
                assert!(keep.len() <= 1);

                let cond = *condition as u32 as u64;

                self.memory_table_lookup_stack_read_cond.assign(
                    ctx,
                    entry.memory_rw_entires[0].start_eid,
                    step.current.eid,
                    entry.memory_rw_entires[0].end_eid,
                    step.current.sp + 1,
                    LocationType::Stack,
                    true,
                    cond,
                )?;

                self.drop_cell.assign(ctx, F::from(*drop as u64))?;

                if keep.len() > 0 {
                    let keep_type: VarType = keep[0].into();

                    self.keep_cell.assign(ctx, F::one())?;
                    self.value_cell.assign(ctx, keep_values[0])?;
                    self.is_i32_cell.assign(ctx, F::from(keep_type as u64))?;
                    if *condition == 0 {
                        self.memory_table_lookup_stack_read_return_value.assign(
                            ctx,
                            entry.memory_rw_entires[1].start_eid,
                            step.current.eid,
                            entry.memory_rw_entires[1].end_eid,
                            step.current.sp + 2,
                            LocationType::Stack,
                            VarType::from(keep[0]) == VarType::I32,
                            keep_values[0],
                        )?;

                        self.memory_table_lookup_stack_write_return_value.assign(
                            ctx,
                            step.current.eid,
                            entry.memory_rw_entires[2].end_eid,
                            step.current.sp + *drop + 2,
                            LocationType::Stack,
                            VarType::from(keep[0]) == VarType::I32,
                            keep_values[0],
                        )?;
                    }
                }

                self.cond_cell.assign(ctx, cond)?;
                self.cond_inv_cell
                    .assign(ctx, F::from(cond).invert().unwrap_or(F::zero()))?;
                self.cond_is_zero_cell
                    .assign(ctx, if cond == 0 { F::one() } else { F::zero() })?;
                self.cond_is_not_zero_cell
                    .assign(ctx, if cond == 0 { F::zero() } else { F::one() })?;
                self.dst_pc_cell.assign(ctx, F::from((*dst_pc) as u64))?;
            }
            _ => unreachable!(),
        }

        Ok(())
    }

    fn sp_diff(&self, meta: &mut VirtualCells<'_, F>) -> Option<Expression<F>> {
        Some(constant_from!(1) + self.cond_is_zero_cell.expr(meta) * self.drop_cell.expr(meta))
    }

    fn mops(&self, meta: &mut VirtualCells<'_, F>) -> Option<Expression<F>> {
        Some(self.cond_is_zero_cell.expr(meta) * self.keep_cell.expr(meta))
    }

    fn memory_writing_ops(&self, entry: &EventTableEntry) -> u32 {
        match &entry.step_info {
            StepInfo::BrIfEqz {
                drop,
                keep,
                keep_values,
                condition,
                ..
            } => {
                if *condition != 0 {
                    0
                } else {
                    keep.len() as u32
                }
            }
            _ => unreachable!(),
        }
    }

    fn next_iid(
        &self,
        meta: &mut VirtualCells<'_, F>,
        common_config: &EventTableCommonConfig<F>,
    ) -> Option<Expression<F>> {
        Some(
            self.cond_is_zero_cell.expr(meta) * self.dst_pc_cell.expr(meta)
                + self.cond_is_not_zero_cell.expr(meta)
                    * (common_config.iid_cell.curr_expr(meta) + constant_from!(1)),
        )
    }
}
