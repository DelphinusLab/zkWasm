use crate::circuits::cell::*;
use crate::circuits::etable::allocator::*;
use crate::circuits::etable::ConstraintBuilder;
use crate::circuits::etable::EventTableCommonArgsConfig;
use crate::circuits::etable::EventTableCommonConfig;
use crate::circuits::etable::EventTableOpcodeConfig;
use crate::circuits::etable::EventTableOpcodeConfigBuilder;
use crate::circuits::utils::step_status::StepStatus;
use crate::circuits::utils::table_entry::EventTableEntryWithMemoryInfo;
use crate::circuits::utils::Context;
use crate::constant;
use crate::constant_from;
use halo2_proofs::arithmetic::FieldExt;
use halo2_proofs::plonk::Error;
use halo2_proofs::plonk::Expression;
use halo2_proofs::plonk::VirtualCells;
use specs::encode::opcode::encode_memory_grow;
use specs::encode::opcode::UniArgEncode;
use specs::etable::EventTableEntry;
use specs::mtable::LocationType;
use specs::step::StepInfo;

pub struct MemoryGrowConfig<F: FieldExt> {
    grow_size_arg: EventTableCommonArgsConfig<F>,

    result: AllocatedU64Cell<F>,
    success: AllocatedBitCell<F>,
    current_maximal_diff: AllocatedCommonRangeCell<F>,

    memory_table_lookup_stack_write: AllocatedMemoryTableLookupWriteCell<F>,
}

pub struct MemoryGrowConfigBuilder {}

impl<F: FieldExt> EventTableOpcodeConfigBuilder<F> for MemoryGrowConfigBuilder {
    fn configure(
        common_config: &EventTableCommonConfig<F>,
        allocator: &mut EventTableCellAllocator<F>,
        constraint_builder: &mut ConstraintBuilder<F>,
    ) -> Box<dyn EventTableOpcodeConfig<F>> {
        let grow_size_arg = common_config.uniarg_configs[0].clone();
        let grow_size = grow_size_arg.value_cell;
        constraint_builder.push(
            "select: uniarg",
            Box::new(move |meta| vec![grow_size_arg.is_i32_cell.expr(meta) - constant_from!(1)]),
        );

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

        let uniarg_configs = common_config.uniarg_configs.clone();
        let memory_table_lookup_stack_write = allocator.alloc_memory_table_lookup_write_cell(
            "op_memory_grow stack write",
            constraint_builder,
            eid,
            move |____| constant_from!(LocationType::Stack as u64),
            move |meta| Self::sp_after_uniarg(sp, &uniarg_configs, meta),
            move |____| constant_from!(1),
            move |meta| result.expr(meta),
            move |____| constant_from!(1),
        );

        Box::new(MemoryGrowConfig {
            grow_size_arg,
            result,
            success,
            current_maximal_diff,
            memory_table_lookup_stack_write,
        })
    }
}

impl<F: FieldExt> EventTableOpcodeConfig<F> for MemoryGrowConfig<F> {
    fn opcode(&self, _meta: &mut VirtualCells<'_, F>) -> Expression<F> {
        encode_memory_grow(UniArgEncode::Reserve)
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

                if let specs::itable::Opcode::MemoryGrow { uniarg, .. } =
                    entry.eentry.get_instruction(step.current.itable).opcode
                {
                    let mut memory_entries = entry.memory_rw_entires.iter();

                    self.grow_size_arg
                        .assign(ctx, uniarg, &mut memory_entries)?;
                } else {
                    unreachable!();
                }

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
        Some(self.success.expr(meta) * self.grow_size_arg.value_cell.expr(meta))
    }

    fn sp_diff(&self, _meta: &mut VirtualCells<'_, F>) -> Option<Expression<F>> {
        Some(constant!(-F::one()))
    }
}
