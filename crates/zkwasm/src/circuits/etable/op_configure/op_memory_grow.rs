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
use halo2_proofs::arithmetic::FieldExt;
use halo2_proofs::plonk::Error;
use halo2_proofs::plonk::Expression;
use halo2_proofs::plonk::VirtualCells;
use num_bigint::BigUint;
use specs::etable::EventTableEntry;
use specs::itable::OpcodeClass;
use specs::itable::OPCODE_CLASS_SHIFT;
use specs::mtable::LocationType;
use specs::step::StepInfo;

pub struct MemoryGrowConfig<F: FieldExt> {
    grow_size: AllocatedU64Cell<F>,
    result: AllocatedU64Cell<F>,
    success: AllocatedBitCell<F>,
    current_maximal_diff: AllocatedCommonRangeCell<F>,

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
        let current_maximal_diff = allocator.alloc_common_range_cell();

        let success = allocator.alloc_bit_cell();

        let current_memory_size = common_config.mpages_cell;
        let maximal_memory_pages = common_config.maximal_memory_pages_cell;

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
                        - maximal_memory_pages.expr(meta))
                        * success.expr(meta),
                ]
            }),
        );

        let eid = common_config.eid_cell;
        let sp = common_config.sp_cell;

        let memory_table_lookup_stack_read = allocator.alloc_memory_table_lookup_read_cell(
            "op_memory_grow stack read",
            constraint_builder,
            eid,
            move |____| constant_from!(LocationType::Stack as u64),
            move |meta| sp.expr(meta) + constant_from!(1),
            move |____| constant_from!(1),
            move |meta| grow_size.expr(meta),
            move |____| constant_from!(1),
        );

        let memory_table_lookup_stack_write = allocator.alloc_memory_table_lookup_write_cell(
            "op_memory_grow stack write",
            constraint_builder,
            eid,
            move |____| constant_from!(LocationType::Stack as u64),
            move |meta| sp.expr(meta) + constant_from!(1),
            move |____| constant_from!(1),
            move |meta| result.expr(meta),
            move |____| constant_from!(1),
        );

        Box::new(MemoryGrowConfig {
            grow_size,
            result,
            success,
            current_maximal_diff,
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
        step: &mut StepStatus<F>,
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

    fn memory_writing_ops(&self, _: &EventTableEntry) -> u32 {
        1
    }

    fn allocated_memory_pages_diff(&self, meta: &mut VirtualCells<'_, F>) -> Option<Expression<F>> {
        Some(self.success.expr(meta) * self.grow_size.expr(meta))
    }
}
