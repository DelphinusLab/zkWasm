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
use crate::constant_from;
use halo2_proofs::arithmetic::FieldExt;
use halo2_proofs::plonk::Error;
use halo2_proofs::plonk::Expression;
use halo2_proofs::plonk::VirtualCells;
use specs::encode::opcode::encode_local_set;
use specs::encode::opcode::UniArgEncode;
use specs::etable::EventTableEntry;
use specs::mtable::LocationType;
use specs::mtable::VarType;
use specs::step::StepInfo;

pub struct LocalSetConfig<F: FieldExt> {
    value_arg: EventTableCommonArgsConfig<F>,
    offset_cell: AllocatedCommonRangeCell<F>,
    memory_table_lookup_stack_write: AllocatedMemoryTableLookupWriteCell<F>,
}

pub struct LocalSetConfigBuilder {}

impl<F: FieldExt> EventTableOpcodeConfigBuilder<F> for LocalSetConfigBuilder {
    fn configure(
        common_config: &EventTableCommonConfig<F>,
        allocator: &mut EventTableCellAllocator<F>,
        constraint_builder: &mut ConstraintBuilder<F>,
    ) -> Box<dyn EventTableOpcodeConfig<F>> {
        let value_arg = common_config.uniarg_configs[0].clone();
        let is_i32_cell = value_arg.is_i32_cell;
        let value_cell = value_arg.value_cell;

        let offset_cell = allocator.alloc_common_range_cell();
        let sp_cell = common_config.sp_cell;
        let eid_cell = common_config.eid_cell;

        let uniarg_configs = common_config.uniarg_configs.clone();
        let memory_table_lookup_stack_write = allocator.alloc_memory_table_lookup_write_cell(
            "op_local_set stack write",
            constraint_builder,
            eid_cell,
            move |____| constant_from!(LocationType::Stack as u64),
            move |meta| {
                Self::sp_after_uniarg(sp_cell, &uniarg_configs, meta) + offset_cell.expr(meta)
            },
            move |meta| is_i32_cell.expr(meta),
            move |meta| value_cell.expr(meta),
            move |____| constant_from!(1),
        );

        Box::new(LocalSetConfig {
            value_arg,
            offset_cell,
            memory_table_lookup_stack_write,
        })
    }
}

impl<F: FieldExt> EventTableOpcodeConfig<F> for LocalSetConfig<F> {
    fn opcode(&self, meta: &mut VirtualCells<'_, F>) -> Expression<F> {
        encode_local_set(
            self.value_arg.is_i32_cell.expr(meta),
            self.offset_cell.expr(meta),
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
            StepInfo::SetLocal {
                vtype,
                depth,
                value,
            } => {
                self.offset_cell.assign(ctx, F::from(*depth as u64))?;

                if let specs::itable::Opcode::LocalSet { uniarg, .. } =
                    entry.eentry.get_instruction(step.current.itable).opcode
                {
                    let mut memory_entries = entry.memory_rw_entires.iter();

                    self.value_arg.assign(ctx, uniarg, &mut memory_entries)?;
                } else {
                    unreachable!();
                }

                self.memory_table_lookup_stack_write.assign(
                    ctx,
                    step.current.eid,
                    entry.memory_rw_entires[1].end_eid,
                    step.current.sp + 1 + depth,
                    LocationType::Stack,
                    *vtype == VarType::I32,
                    *value,
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
}
