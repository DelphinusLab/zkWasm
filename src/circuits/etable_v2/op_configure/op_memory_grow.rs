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
use num_bigint::BigUint;
use specs::{
    encode::{frame_table::encode_frame_table_entry, opcode::encode_call},
    etable::EventTableEntry,
    itable::{OpcodeClass, OPCODE_ARG0_SHIFT, OPCODE_ARG1_SHIFT, OPCODE_CLASS_SHIFT},
    mtable::{LocationType, VarType},
    step::StepInfo,
};

pub struct MemoryGrowConfig<F: FieldExt> {
    grow_size: AllocatedU64Cell<F>,
    result: AllocatedU64Cell<F>,
    success: AllocatedBitCell<F>,
    current_maximal_diff: AllocatedCommonRangeCell<F>,
    fail_maximal_diff: AllocatedU64Cell<F>,

    memory_table_lookup_stack_read: AllocatedMemoryTableLookupReadCell<F>,
    memory_table_lookup_stack_write: AllocatedMemoryTableLookupWriteCell<F>,
}

pub struct MemoryGrowConfigBuilder {}

impl<F: FieldExt> EventTableOpcodeConfigBuilder<F> for MemoryGrowConfigBuilder {
    fn configure(
        common_config: &EventTableCommonConfig<F>,
        allocator: &mut EventTableCellAllocator<F>,
        constraint_builder: &mut ConstraintBuilder<F>,
    ) -> Box<dyn EventTableOpcodeConfig<F>> {
        let grow_size = allocator.alloc_u64_cell();
        let result = allocator.alloc_u64_cell();
        let fail_maximal_diff = allocator.alloc_u64_cell();
        let current_maximal_diff = allocator.alloc_common_range_cell();
        let current_memory_size = common_config.mpages_cell;

        let success = allocator.alloc_bit_cell();

        let maximal_memory_pages = common_config.circuit_configure.maximal_memory_pages;

        constraint_builder.push(
            "memory_grow: return value",
            Box::new(move |meta| {
                vec![
                    result.expr(meta)
                        - (constant_from!(u32::MAX)
                            + success.expr(meta)
                                * (current_memory_size.expr(meta) - constant_from!(u32::MAX))),
                ]
            }),
        );

        constraint_builder.push(
            "memory_grow: updated memory size should less or equal than maximal memory size",
            Box::new(move |meta| {
                vec![
                    (current_memory_size.expr(meta)
                        + grow_size.expr(meta)
                        + current_maximal_diff.expr(meta)
                        - constant_from!(maximal_memory_pages))
                        * success.expr(meta),
                ]
            }),
        );

        constraint_builder.push(
            "memory_grow: fail only on exceed limit",
            Box::new(move |meta| {
                vec![
                    (success.expr(meta) - constant_from!(1))
                        * (current_memory_size.expr(meta) + grow_size.expr(meta)
                            - constant_from!(maximal_memory_pages)
                            - constant_from!(1)
                            - fail_maximal_diff.expr(meta)),
                ]
            }),
        );

        let eid = common_config.eid_cell;
        let sp = common_config.sp_cell;

        let memory_table_lookup_stack_read = allocator.alloc_memory_table_lookup_read_cell(
            "op_test stack read",
            constraint_builder,
            eid,
            move |meta| constant_from!(LocationType::Stack as u64),
            move |meta| sp.expr(meta) + constant_from!(1),
            move |meta| constant_from!(1),
            move |meta| grow_size.expr(meta),
            move |meta| constant_from!(1),
        );

        let memory_table_lookup_stack_write = allocator.alloc_memory_table_lookup_write_cell(
            "op_test stack write",
            constraint_builder,
            eid,
            move |meta| constant_from!(LocationType::Stack as u64),
            move |meta| sp.expr(meta) + constant_from!(1),
            move |meta| constant_from!(1),
            move |meta| result.expr(meta),
            move |meta| constant_from!(1),
        );

        Box::new(MemoryGrowConfig {
            grow_size,
            result,
            success,
            current_maximal_diff,
            fail_maximal_diff,
            memory_table_lookup_stack_read,
            memory_table_lookup_stack_write,
        })
    }
}

impl<F: FieldExt> EventTableOpcodeConfig<F> for MemoryGrowConfig<F> {
    fn opcode(&self, _meta: &mut VirtualCells<'_, F>) -> Expression<F> {
        constant!(bn_to_field(
            &(BigUint::from(OpcodeClass::MemoryGrow as u64) << OPCODE_CLASS_SHIFT)
        ))
    }

    fn assign(
        &self,
        ctx: &mut Context<'_, F>,
        step: &StepStatus,
        entry: &EventTableEntryWithMemoryInfo,
    ) -> Result<(), Error> {
        match &entry.eentry.step_info {
            StepInfo::MemoryGrow { grow_size, result } => {
                let success = *result != -1;

                self.grow_size.assign(ctx, *grow_size as u64)?;
                self.result.assign(ctx, *result as u32 as u64)?;
                self.success.assign_bool(ctx, success)?;
                if success {
                    self.current_maximal_diff.assign(
                        ctx,
                        F::from(
                            (step.configure_table.maximal_memory_pages
                                - (step.current.allocated_memory_pages + *grow_size as u32))
                                as u64,
                        ),
                    )?;
                } else {
                    self.fail_maximal_diff.assign(
                        ctx,
                        step.current.allocated_memory_pages as u64 + *grow_size as u32 as u64
                            - step.configure_table.maximal_memory_pages as u64
                            - 1,
                    )?;
                }

                self.memory_table_lookup_stack_read.assign(
                    ctx,
                    entry.memory_rw_entires[0].start_eid,
                    step.current.eid,
                    entry.memory_rw_entires[0].end_eid,
                    step.current.sp + 1,
                    LocationType::Stack,
                    true,
                    *grow_size as u32 as u64,
                )?;

                self.memory_table_lookup_stack_write.assign(
                    ctx,
                    step.current.eid,
                    entry.memory_rw_entires[1].end_eid,
                    step.current.sp + 1,
                    LocationType::Stack,
                    true,
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

    fn memory_writing_ops(&self, entry: &EventTableEntry) -> u32 {
        1
    }

    fn allocated_memory_pages_diff(&self, meta: &mut VirtualCells<'_, F>) -> Option<Expression<F>> {
        Some(self.success.expr(meta) * self.grow_size.expr(meta))
    }
}
