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
    constant, constant_from,
};
use halo2_proofs::{
    arithmetic::FieldExt,
    plonk::{Error, Expression, VirtualCells},
};
use num_bigint::BigUint;
use specs::{
    etable::EventTableEntry,
    itable::{OpcodeClass, OPCODE_ARG0_SHIFT, OPCODE_CLASS_SHIFT},
    mtable::{LocationType, VarType},
    step::StepInfo, encode::opcode::encode_global_get,
};

pub struct GlobalGetConfig<F: FieldExt> {
    idx_cell: AllocatedCommonRangeCell<F>,
    is_i32_cell: AllocatedBitCell<F>,
    value_cell: AllocatedU64Cell<F>,
    memory_table_lookup_global_read: AllocatedMemoryTableLookupReadCell<F>,
    memory_table_lookup_stack_write: AllocatedMemoryTableLookupWriteCell<F>,
}

pub struct GlobalGetConfigBuilder {}

impl<F: FieldExt> EventTableOpcodeConfigBuilder<F> for GlobalGetConfigBuilder {
    fn configure(
        common_config: &EventTableCommonConfig<F>,
        allocator: &mut EventTableCellAllocator<F>,
        constraint_builder: &mut ConstraintBuilder<F>,
    ) -> Box<dyn EventTableOpcodeConfig<F>> {
        let is_i32_cell = allocator.alloc_bit_cell();
        let idx_cell = allocator.alloc_common_range_cell();
        let value_cell = allocator.alloc_u64_cell();

        let sp_cell = common_config.sp_cell;
        let eid_cell = common_config.eid_cell;

        let memory_table_lookup_global_read = allocator.alloc_memory_table_lookup_read_cell(
            "op_global_get global read",
            constraint_builder,
            eid_cell,
            move |_| constant_from!(LocationType::Global as u64),
            move |meta| idx_cell.expr(meta),
            move |meta| is_i32_cell.expr(meta),
            move |meta| value_cell.u64_cell.expr(meta),
            move |_| constant_from!(1),
        );

        let memory_table_lookup_stack_write = allocator.alloc_memory_table_lookup_write_cell(
            "op_global_get stack write",
            constraint_builder,
            eid_cell,
            move |_| constant_from!(LocationType::Stack as u64),
            move |meta| sp_cell.expr(meta),
            move |meta| is_i32_cell.expr(meta),
            move |meta| value_cell.u64_cell.expr(meta),
            move |_| constant_from!(1),
        );

        Box::new(GlobalGetConfig {
            idx_cell,
            is_i32_cell,
            value_cell,
            memory_table_lookup_global_read,
            memory_table_lookup_stack_write,
        })
    }
}

impl<F: FieldExt> EventTableOpcodeConfig<F> for GlobalGetConfig<F> {
    fn opcode(&self, meta: &mut VirtualCells<'_, F>) -> Expression<F> {
        encode_global_get(self.idx_cell.expr(meta))
    }

    fn assign(
        &self,
        ctx: &mut Context<'_, F>,
        step: &StepStatus,
        entry: &EventTableEntryWithMemoryInfo,
    ) -> Result<(), Error> {
        match &entry.eentry.step_info {
            StepInfo::GetGlobal {
                idx, vtype, value, ..
            } => {
                self.idx_cell.assign(ctx, F::from(*idx as u64))?;
                self.is_i32_cell.assign(ctx, F::from(*vtype as u64))?;
                self.value_cell.assign(ctx, *value)?;

                self.memory_table_lookup_global_read.assign(
                    ctx,
                    entry.memory_rw_entires[0].start_eid,
                    step.current.eid,
                    entry.memory_rw_entires[0].end_eid,
                    *idx,
                    LocationType::Global,
                    *vtype == VarType::I32,
                    *value,
                )?;

                self.memory_table_lookup_stack_write.assign(
                    ctx,
                    step.current.eid,
                    entry.memory_rw_entires[0].end_eid,
                    step.current.sp,
                    LocationType::Stack,
                    *vtype == VarType::I32,
                    *value,
                )?;

                Ok(())
            }

            _ => unreachable!(),
        }
    }

    fn sp_diff(&self, _meta: &mut VirtualCells<'_, F>) -> Option<Expression<F>> {
        Some(constant!(-F::one()))
    }

    fn mops(&self, _meta: &mut VirtualCells<'_, F>) -> Option<Expression<F>> {
        Some(constant_from!(1))
    }

    fn memory_writing_ops(&self, entry: &EventTableEntry) -> u32 {
        1
    }
}
