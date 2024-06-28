use crate::circuits::cell::*;
use crate::circuits::etable::allocator::*;
use crate::circuits::etable::ConstraintBuilder;
use crate::circuits::etable::EventTableCommonConfig;
use crate::circuits::etable::EventTableOpcodeConfig;
use crate::circuits::etable::EventTableOpcodeConfigBuilder;
use crate::circuits::utils::step_status::StepStatus;
use crate::circuits::utils::table_entry::EventTableEntryWithMemoryInfo;
use crate::circuits::utils::Context;
use crate::constant_from;
use halo2_proofs::arithmetic::FieldExt;
use halo2_proofs::plonk::Error;
use halo2_proofs::plonk::Expression;
use halo2_proofs::plonk::VirtualCells;
use num_bigint::BigUint;
use specs::encode::br_table::encode_br_table_entry;
use specs::encode::opcode::encode_br_table;
use specs::etable::EventTableEntry;
use specs::mtable::LocationType;
use specs::mtable::VarType;
use specs::step::StepInfo;

pub struct BrTableConfig<F: FieldExt> {
    keep: AllocatedBitCell<F>,
    keep_is_i32: AllocatedBitCell<F>,
    keep_value: AllocatedU64Cell<F>,
    drop: AllocatedCommonRangeCell<F>,
    dst_iid: AllocatedCommonRangeCell<F>,

    expected_index: AllocatedU64Cell<F>,
    effective_index: AllocatedCommonRangeCell<F>,
    targets_len: AllocatedCommonRangeCell<F>,
    is_out_of_bound: AllocatedBitCell<F>,
    is_not_out_of_bound: AllocatedBitCell<F>,
    diff: AllocatedU64Cell<F>,

    br_table_lookup: AllocatedUnlimitedCell<F>,

    memory_table_lookup_stack_read_index: AllocatedMemoryTableLookupReadCell<F>,
    memory_table_lookup_stack_read_return_value: AllocatedMemoryTableLookupReadCell<F>,
    memory_table_lookup_stack_write_return_value: AllocatedMemoryTableLookupWriteCell<F>,
}

pub struct BrTableConfigBuilder;

impl<F: FieldExt> EventTableOpcodeConfigBuilder<F> for BrTableConfigBuilder {
    fn configure(
        common_config: &EventTableCommonConfig<F>,
        allocator: &mut EventTableCellAllocator<F>,
        constraint_builder: &mut ConstraintBuilder<F>,
    ) -> Box<dyn EventTableOpcodeConfig<F>> {
        let keep = allocator.alloc_bit_cell();
        let keep_is_i32 = allocator.alloc_bit_cell();
        let keep_value = allocator.alloc_u64_cell();
        let drop = allocator.alloc_common_range_cell();
        let dst_iid = allocator.alloc_common_range_cell();
        let expected_index = allocator.alloc_u64_cell();
        let effective_index = allocator.alloc_common_range_cell();
        let targets_len = allocator.alloc_common_range_cell();
        let is_out_of_bound = allocator.alloc_bit_cell();
        let is_not_out_of_bound = allocator.alloc_bit_cell();
        let diff = allocator.alloc_u64_cell();

        constraint_builder.push(
            "op_br_table oob",
            Box::new(move |meta| {
                vec![
                    /* is_out_of_bound \/ is_not_out_of_bound */
                    is_not_out_of_bound.expr(meta) + is_out_of_bound.expr(meta) - constant_from!(1),
                    /* is_out_of_bound -> expected >= targets_len */
                    is_out_of_bound.expr(meta)
                        * (targets_len.expr(meta) + diff.expr(meta) - expected_index.expr(meta)),
                    /* !is_out_of_bound -> expected_index < targets_len */
                    is_not_out_of_bound.expr(meta)
                        * (expected_index.expr(meta) + diff.expr(meta) + constant_from!(1)
                            - targets_len.expr(meta)),
                ]
            }),
        );

        constraint_builder.push(
            "op_br_table effective_index",
            Box::new(move |meta| {
                vec![
                    is_out_of_bound.expr(meta)
                        * (targets_len.expr(meta) - constant_from!(1) - effective_index.expr(meta)),
                    is_not_out_of_bound.expr(meta)
                        * (expected_index.expr(meta) - effective_index.expr(meta)),
                ]
            }),
        );

        let br_table_lookup = common_config.brtable_lookup_cell;
        let fid = common_config.fid_cell;
        let iid = common_config.iid_cell;

        constraint_builder.push(
            "op_br_table br_table_lookup",
            Box::new(move |meta| {
                vec![
                    br_table_lookup.expr(meta)
                        - encode_br_table_entry(
                            fid.expr(meta),
                            iid.expr(meta),
                            effective_index.expr(meta),
                            drop.expr(meta),
                            keep.expr(meta),
                            dst_iid.expr(meta),
                        ),
                ]
            }),
        );

        let eid = common_config.eid_cell;
        let sp = common_config.sp_cell;

        let memory_table_lookup_stack_read_index = allocator.alloc_memory_table_lookup_read_cell(
            "op_br_table stack read index",
            constraint_builder,
            eid,
            move |____| constant_from!(LocationType::Stack as u64),
            move |meta| sp.expr(meta) + constant_from!(1),
            move |____| constant_from!(1),
            move |meta| expected_index.expr(meta),
            move |____| constant_from!(1),
        );

        let memory_table_lookup_stack_read_return_value = allocator
            .alloc_memory_table_lookup_read_cell(
                "op_br_table stack read index",
                constraint_builder,
                eid,
                move |____| constant_from!(LocationType::Stack as u64),
                move |meta| sp.expr(meta) + constant_from!(2),
                move |meta| keep_is_i32.expr(meta),
                move |meta| keep_value.expr(meta),
                move |meta| keep.expr(meta),
            );

        let memory_table_lookup_stack_write_return_value = allocator
            .alloc_memory_table_lookup_write_cell(
                "op_br stack write",
                constraint_builder,
                eid,
                move |____| constant_from!(LocationType::Stack as u64),
                move |meta| sp.expr(meta) + drop.expr(meta) + constant_from!(2),
                move |meta| keep_is_i32.expr(meta),
                move |meta| keep_value.expr(meta),
                move |meta| keep.expr(meta),
            );

        Box::new(BrTableConfig {
            keep,
            keep_is_i32,
            keep_value,
            drop,
            dst_iid,
            expected_index,
            effective_index,
            targets_len,
            is_out_of_bound,
            is_not_out_of_bound,
            diff,
            br_table_lookup,
            memory_table_lookup_stack_read_index,
            memory_table_lookup_stack_read_return_value,
            memory_table_lookup_stack_write_return_value,
        })
    }
}

impl<F: FieldExt> EventTableOpcodeConfig<F> for BrTableConfig<F> {
    fn opcode(&self, meta: &mut VirtualCells<'_, F>) -> Expression<F> {
        encode_br_table(self.targets_len.expr(meta))
    }

    fn assign(
        &self,
        ctx: &mut Context<'_, F>,
        step: &mut StepStatus<F>,
        entry: &EventTableEntryWithMemoryInfo,
    ) -> Result<(), Error> {
        match &entry.eentry.step_info {
            StepInfo::BrTable {
                index,
                dst_pc,
                drop,
                keep,
                keep_values,
            } => {
                assert!(keep.len() <= 1);

                let index = *index as u32 as u64;
                let targets = match &entry.eentry.get_instruction(step.current.itable).opcode {
                    specs::itable::Opcode::BrTable { targets } => targets.clone(),
                    _ => unreachable!(),
                };
                let targets_len = targets.len() as u64;

                self.drop.assign(ctx, F::from(*drop as u64))?;
                self.dst_iid.assign(ctx, F::from(*dst_pc as u64))?;

                self.memory_table_lookup_stack_read_index.assign(
                    ctx,
                    entry.memory_rw_entires[0].start_eid,
                    step.current.eid,
                    entry.memory_rw_entires[0].end_eid,
                    step.current.sp + 1,
                    LocationType::Stack,
                    true,
                    index,
                )?;

                if !keep.is_empty() {
                    let keep_type: VarType = keep[0].into();

                    self.keep.assign(ctx, F::one())?;
                    self.keep_value.assign(ctx, keep_values[0])?;
                    self.keep_is_i32
                        .assign_bool(ctx, keep_type == VarType::I32)?;

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
                        step.current.sp + drop + 2,
                        LocationType::Stack,
                        VarType::from(keep[0]) == VarType::I32,
                        keep_values[0],
                    )?;
                }

                self.targets_len.assign(ctx, F::from(targets_len))?;

                let effective_index = if index < targets_len {
                    index
                } else {
                    targets_len - 1
                };
                self.expected_index.assign(ctx, index)?;
                self.effective_index.assign(ctx, F::from(effective_index))?;
                self.is_out_of_bound
                    .assign_bool(ctx, index != effective_index)?;
                self.is_not_out_of_bound
                    .assign_bool(ctx, index == effective_index)?;
                self.diff.assign(
                    ctx,
                    if index < targets_len {
                        targets_len - index - 1
                    } else {
                        index - targets_len
                    },
                )?;

                self.br_table_lookup.assign_bn(
                    ctx,
                    &encode_br_table_entry(
                        BigUint::from(step.current.fid),
                        BigUint::from(step.current.iid),
                        BigUint::from(effective_index),
                        BigUint::from(*drop),
                        BigUint::from(keep.len()),
                        BigUint::from(*dst_pc),
                    ),
                )?;
            }
            _ => unreachable!(),
        }

        Ok(())
    }

    fn sp_diff(&self, meta: &mut VirtualCells<'_, F>) -> Option<Expression<F>> {
        Some(self.drop.expr(meta) + constant_from!(1))
    }

    fn next_iid(
        &self,
        meta: &mut VirtualCells<'_, F>,
        _common_config: &EventTableCommonConfig<F>,
    ) -> Option<Expression<F>> {
        Some(self.dst_iid.expr(meta))
    }

    fn mops(&self, meta: &mut VirtualCells<'_, F>) -> Option<Expression<F>> {
        Some(self.keep.expr(meta))
    }

    fn memory_writing_ops(&self, entry: &EventTableEntry) -> u32 {
        match &entry.step_info {
            StepInfo::BrTable { keep, .. } => keep.len() as u32,
            _ => unreachable!(),
        }
    }
}
