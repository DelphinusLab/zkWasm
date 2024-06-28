use halo2_proofs::arithmetic::FieldExt;
use halo2_proofs::plonk::Error;
use halo2_proofs::plonk::Expression;
use halo2_proofs::plonk::VirtualCells;
use num_bigint::BigUint;
use specs::etable::EventTableEntry;
use specs::host_function::HostPlugin;
use specs::itable::OpcodeClass;
use specs::itable::OPCODE_CLASS_SHIFT;
use specs::mtable::LocationType;
use specs::step::StepInfo;

use crate::circuits::cell::AllocatedBitCell;
use crate::circuits::cell::AllocatedUnlimitedCell;
use crate::circuits::cell::CellExpression;
use crate::circuits::etable::allocator::AllocatedMemoryTableLookupReadCell;
use crate::circuits::etable::allocator::AllocatedMemoryTableLookupWriteCell;
use crate::circuits::etable::allocator::EventTableCellAllocator;
use crate::circuits::etable::constraint_builder::ConstraintBuilder;
use crate::circuits::etable::EventTableCommonConfig;
use crate::circuits::etable::EventTableOpcodeConfig;
use crate::circuits::utils::bn_to_field;
use crate::circuits::utils::step_status::StepStatus;
use crate::circuits::utils::table_entry::EventTableEntryWithMemoryInfo;
use crate::circuits::utils::Context;
use crate::constant;
use crate::constant_from;
use crate::constant_from_bn;
use crate::foreign::context::Op;
use crate::foreign::EventTableForeignCallConfigBuilder;
use crate::foreign::InternalHostPluginBuilder;

use super::circuits::CONTEXT_FOREIGN_TABLE_KEY;

pub struct ETableContextHelperTableConfig<F: FieldExt> {
    plugin_index: u64,

    is_context_in_op: AllocatedBitCell<F>,
    is_context_out_op: AllocatedBitCell<F>,
    context_input_index_for_lookup: AllocatedUnlimitedCell<F>,
    context_output_index_for_lookup: AllocatedUnlimitedCell<F>,
    input_value: AllocatedUnlimitedCell<F>,
    output_value: AllocatedUnlimitedCell<F>,

    lookup_read_stack: AllocatedMemoryTableLookupReadCell<F>,
    lookup_write_stack: AllocatedMemoryTableLookupWriteCell<F>,
}

pub struct ETableContextHelperTableConfigBuilder {
    index: usize,
}

impl InternalHostPluginBuilder for ETableContextHelperTableConfigBuilder {
    fn new(index: usize) -> Self {
        Self { index }
    }
}
impl<F: FieldExt> EventTableForeignCallConfigBuilder<F> for ETableContextHelperTableConfigBuilder {
    fn configure(
        self,
        common_config: &EventTableCommonConfig<F>,
        allocator: &mut EventTableCellAllocator<F>,
        constraint_builder: &mut ConstraintBuilder<F>,
        lookup_cells: &mut (impl Iterator<Item = AllocatedUnlimitedCell<F>> + Clone),
    ) -> Box<dyn EventTableOpcodeConfig<F>> {
        let is_context_in_op = allocator.alloc_bit_cell();
        let is_context_out_op = allocator.alloc_bit_cell();
        let context_input_index_for_lookup = lookup_cells.next().unwrap();
        let context_output_index_for_lookup = lookup_cells.next().unwrap();

        let input_value = lookup_cells.next().unwrap();
        let output_value = lookup_cells.next().unwrap();

        let sp = common_config.sp_cell;

        let context_input_index = common_config.context_input_index_cell;
        let context_output_index = common_config.context_output_index_cell;

        let lookup_read_stack = allocator.alloc_memory_table_lookup_read_cell(
            "context stack read",
            constraint_builder,
            common_config.eid_cell,
            move |____| constant_from!(LocationType::Stack as u64),
            move |meta| sp.expr(meta) + constant_from!(1),
            move |____| constant_from!(0),
            move |meta| output_value.expr(meta),
            move |meta| is_context_out_op.expr(meta),
        );
        let lookup_write_stack = allocator.alloc_memory_table_lookup_write_cell(
            "context stack write",
            constraint_builder,
            common_config.eid_cell,
            move |____| constant_from!(LocationType::Stack as u64),
            move |meta| sp.expr(meta),
            move |____| constant_from!(0),
            move |meta| input_value.expr(meta),
            move |meta| is_context_in_op.expr(meta),
        );

        constraint_builder.push(
            "context sel",
            Box::new(move |meta| {
                vec![is_context_in_op.expr(meta) + is_context_out_op.expr(meta) - constant_from!(1)]
            }),
        );

        constraint_builder.push(
            "context lookup",
            Box::new(move |meta| {
                vec![
                    is_context_in_op.expr(meta)
                        * (context_input_index_for_lookup.expr(meta)
                            - context_input_index.expr(meta)),
                    is_context_out_op.expr(meta)
                        * (context_output_index_for_lookup.expr(meta)
                            - context_output_index.expr(meta)),
                ]
            }),
        );

        constraint_builder.lookup(
            CONTEXT_FOREIGN_TABLE_KEY,
            "lookup context cont table",
            Box::new(move |meta| {
                vec![
                    context_input_index_for_lookup.expr(meta),
                    input_value.expr(meta),
                    context_output_index_for_lookup.expr(meta),
                    output_value.expr(meta),
                ]
            }),
        );

        Box::new(ETableContextHelperTableConfig {
            plugin_index: self.index as u64,
            is_context_in_op,
            is_context_out_op,
            input_value,
            output_value,
            lookup_read_stack,
            lookup_write_stack,
            context_input_index_for_lookup,
            context_output_index_for_lookup,
        })
    }
}

impl<F: FieldExt> EventTableOpcodeConfig<F> for ETableContextHelperTableConfig<F> {
    fn opcode(&self, meta: &mut VirtualCells<'_, F>) -> Expression<F> {
        constant_from_bn!(
            &(BigUint::from(OpcodeClass::ForeignPluginStart as u64 + self.plugin_index)
                << OPCODE_CLASS_SHIFT)
        ) + self.is_context_out_op.expr(meta)
    }

    fn assign(
        &self,
        ctx: &mut Context<'_, F>,
        step: &mut StepStatus<F>,
        entry: &EventTableEntryWithMemoryInfo,
    ) -> Result<(), Error> {
        match &entry.eentry.step_info {
            StepInfo::CallHost {
                plugin,
                args,
                ret_val,
                op_index_in_plugin,
                ..
            } => {
                assert_eq!(*plugin, HostPlugin::Context);

                if *op_index_in_plugin == Op::ReadContext as usize {
                    let value = ret_val.unwrap();

                    self.input_value.assign(ctx, F::from(value))?;
                    self.lookup_write_stack.assign(
                        ctx,
                        step.current.eid,
                        entry.memory_rw_entires[0].end_eid,
                        step.current.sp,
                        LocationType::Stack,
                        false,
                        value,
                    )?;
                    self.is_context_in_op.assign_bool(ctx, true)?;
                    self.context_input_index_for_lookup
                        .assign(ctx, F::from(step.current.context_in_index as u64))?;

                    Ok(())
                } else {
                    let value = *args.first().unwrap();

                    self.output_value.assign(ctx, F::from(value))?;
                    self.lookup_read_stack.assign(
                        ctx,
                        entry.memory_rw_entires[0].start_eid,
                        step.current.eid,
                        entry.memory_rw_entires[0].end_eid,
                        step.current.sp + 1,
                        LocationType::Stack,
                        false,
                        value,
                    )?;
                    self.is_context_out_op.assign_bool(ctx, true)?;
                    self.context_output_index_for_lookup
                        .assign(ctx, F::from(step.current.context_out_index as u64))?;

                    Ok(())
                }
            }
            _ => unreachable!(),
        }
    }

    fn mops(&self, meta: &mut VirtualCells<'_, F>) -> Option<Expression<F>> {
        Some(self.is_context_in_op.expr(meta))
    }

    fn memory_writing_ops(&self, entry: &EventTableEntry) -> u32 {
        match &entry.step_info {
            StepInfo::CallHost {
                plugin,
                op_index_in_plugin,
                ..
            } => {
                assert_eq!(*plugin, HostPlugin::Context);

                (*op_index_in_plugin == Op::ReadContext as usize) as u32
            }
            _ => unreachable!(),
        }
    }

    fn sp_diff(&self, meta: &mut VirtualCells<'_, F>) -> Option<Expression<F>> {
        /*
         * context_out: sp + 1
         * context_in: sp - 1
         */
        Some(constant!(-F::one()) + constant_from!(2) * self.is_context_out_op.expr(meta))
    }

    fn is_context_input_op(&self, entry: &EventTableEntry) -> bool {
        match &entry.step_info {
            StepInfo::CallHost {
                plugin,
                op_index_in_plugin,
                ..
            } => {
                assert_eq!(*plugin, HostPlugin::Context);

                *op_index_in_plugin == Op::ReadContext as usize
            }
            _ => unreachable!(),
        }
    }

    fn context_input_index_increase(
        &self,
        meta: &mut VirtualCells<'_, F>,
        _common_config: &EventTableCommonConfig<F>,
    ) -> Option<Expression<F>> {
        Some(self.is_context_in_op.expr(meta))
    }

    fn is_context_output_op(&self, entry: &EventTableEntry) -> bool {
        match &entry.step_info {
            StepInfo::CallHost {
                plugin,
                op_index_in_plugin,
                ..
            } => {
                assert_eq!(*plugin, HostPlugin::Context);

                *op_index_in_plugin == Op::WriteContext as usize
            }
            _ => unreachable!(),
        }
    }

    fn context_output_index_increase(
        &self,
        meta: &mut VirtualCells<'_, F>,
        _common_config: &EventTableCommonConfig<F>,
    ) -> Option<Expression<F>> {
        Some(self.is_context_out_op.expr(meta))
    }
}
