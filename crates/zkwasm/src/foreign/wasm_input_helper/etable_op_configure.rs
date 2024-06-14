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
use specs::mtable::VarType;
use specs::step::StepInfo;

use super::circuits::WASM_INPUT_FOREIGN_TABLE_KEY;

use crate::circuits::cell::AllocatedBitCell;
use crate::circuits::cell::AllocatedU64Cell;
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
use crate::constant_from;
use crate::constant_from_bn;
use crate::foreign::wasm_input_helper::Op;
use crate::foreign::EventTableForeignCallConfigBuilder;
use crate::foreign::InternalHostPluginBuilder;

pub struct ETableWasmInputHelperTableConfig<F: FieldExt> {
    plugin_index: usize,
    is_wasm_input_op: AllocatedBitCell<F>,
    is_wasm_output_op: AllocatedBitCell<F>,

    is_public: AllocatedBitCell<F>,
    value: AllocatedU64Cell<F>,

    enable_input_table_lookup: AllocatedBitCell<F>,
    public_input_index_for_lookup: AllocatedUnlimitedCell<F>,
    value_for_lookup: AllocatedUnlimitedCell<F>,

    lookup_read_stack: AllocatedMemoryTableLookupReadCell<F>,
    lookup_write_stack: AllocatedMemoryTableLookupWriteCell<F>,
}

pub struct ETableWasmInputHelperTableConfigBuilder {
    index: usize,
}

impl InternalHostPluginBuilder for ETableWasmInputHelperTableConfigBuilder {
    fn new(index: usize) -> Self {
        Self { index }
    }
}

impl<F: FieldExt> EventTableForeignCallConfigBuilder<F>
    for ETableWasmInputHelperTableConfigBuilder
{
    fn configure(
        self,
        common_config: &EventTableCommonConfig<F>,
        allocator: &mut EventTableCellAllocator<F>,
        constraint_builder: &mut ConstraintBuilder<F>,
        lookup_cells: &mut (impl Iterator<Item = AllocatedUnlimitedCell<F>> + Clone),
    ) -> Box<dyn EventTableOpcodeConfig<F>> {
        let eid = common_config.eid_cell;
        let sp = common_config.sp_cell;
        let public_input_index = common_config.input_index_cell;

        let is_wasm_input_op: AllocatedBitCell<F> = allocator.alloc_bit_cell();
        let is_wasm_output_op: AllocatedBitCell<F> = allocator.alloc_bit_cell();

        let is_public = allocator.alloc_bit_cell();
        let value = allocator.alloc_u64_cell();

        let enable_input_table_lookup = allocator.alloc_bit_cell();
        let public_input_index_for_lookup = lookup_cells.next().unwrap();
        let value_for_lookup = lookup_cells.next().unwrap();

        let lookup_read_stack = allocator.alloc_memory_table_lookup_read_cell(
            "wasm input stack read",
            constraint_builder,
            eid,
            move |____| constant_from!(LocationType::Stack as u64),
            move |meta| sp.expr(meta) + constant_from!(1),
            move |meta| is_wasm_input_op.expr(meta),
            move |meta| {
                is_public.expr(meta) * is_wasm_input_op.expr(meta)
                    + value.u64_cell.expr(meta) * is_wasm_output_op.expr(meta)
            },
            move |____| constant_from!(1),
        );
        let lookup_write_stack = allocator.alloc_memory_table_lookup_write_cell(
            "wasm input stack write",
            constraint_builder,
            eid,
            move |____| constant_from!(LocationType::Stack as u64),
            move |meta| sp.expr(meta) + constant_from!(1),
            move |____| constant_from!(0),
            move |meta| value.u64_cell.expr(meta),
            move |meta| is_wasm_input_op.expr(meta),
        );

        constraint_builder.push(
            "wasm_input_helper: sel",
            Box::new(move |meta| {
                vec![is_wasm_input_op.expr(meta) + is_wasm_output_op.expr(meta) - constant_from!(1)]
            }),
        );

        constraint_builder.push(
            "enable lookup bit",
            Box::new(move |meta| {
                vec![
                    enable_input_table_lookup.expr(meta)
                        - (is_public.expr(meta) + is_wasm_output_op.expr(meta)),
                ]
            }),
        );

        constraint_builder.push(
            "wasm input lookup aux",
            Box::new(move |meta| {
                vec![
                    enable_input_table_lookup.expr(meta) * public_input_index.expr(meta)
                        - public_input_index_for_lookup.expr(meta),
                    enable_input_table_lookup.expr(meta) * value.expr(meta)
                        - value_for_lookup.expr(meta),
                ]
            }),
        );

        constraint_builder.lookup(
            WASM_INPUT_FOREIGN_TABLE_KEY,
            "lookup input table",
            Box::new(move |meta| {
                vec![
                    public_input_index_for_lookup.expr(meta),
                    value_for_lookup.expr(meta),
                ]
            }),
        );

        Box::new(ETableWasmInputHelperTableConfig {
            plugin_index: self.index,
            is_wasm_input_op,
            is_wasm_output_op,
            is_public,
            value,
            enable_input_table_lookup,
            public_input_index_for_lookup,
            value_for_lookup,
            lookup_read_stack,
            lookup_write_stack,
        })
    }
}

impl<F: FieldExt> EventTableOpcodeConfig<F> for ETableWasmInputHelperTableConfig<F> {
    fn sp_diff(&self, meta: &mut VirtualCells<'_, F>) -> Option<Expression<F>> {
        Some(self.is_wasm_output_op.expr(meta))
    }

    fn opcode(&self, meta: &mut VirtualCells<'_, F>) -> Expression<F> {
        constant_from_bn!(
            &(BigUint::from(OpcodeClass::ForeignPluginStart as u64 + self.plugin_index as u64)
                << OPCODE_CLASS_SHIFT)
        ) + self.is_wasm_output_op.expr(meta)
    }

    fn assign(
        &self,
        ctx: &mut Context<'_, F>,
        step: &mut StepStatus<F>,
        entry: &EventTableEntryWithMemoryInfo,
    ) -> Result<(), Error> {
        match &entry.eentry.step_info {
            StepInfo::CallHost {
                args,
                ret_val,
                signature,
                op_index_in_plugin,
                ..
            } => {
                if *op_index_in_plugin == Op::WasmInput as usize {
                    let arg_type: VarType = (*signature.params.get(0).unwrap()).into();
                    let ret_type: VarType = signature.return_type.unwrap().into();

                    assert_eq!(args.len(), 1);
                    assert_eq!(arg_type, VarType::I32);
                    assert_eq!(ret_type, VarType::I64);

                    let is_public = *args.get(0).unwrap() == 1;
                    let value = ret_val.unwrap();

                    self.is_wasm_input_op.assign_bool(ctx, true)?;
                    self.is_public.assign_bool(ctx, is_public)?;
                    self.value.assign(ctx, value)?;

                    self.enable_input_table_lookup.assign_bool(ctx, is_public)?;
                    self.public_input_index_for_lookup.assign(
                        ctx,
                        (is_public as u64 * step.current.host_public_inputs as u64).into(),
                    )?;
                    self.value_for_lookup
                        .assign(ctx, ((is_public as u64) * value).into())?;

                    self.lookup_read_stack.assign(
                        ctx,
                        entry.memory_rw_entires[0].start_eid,
                        step.current.eid,
                        entry.memory_rw_entires[0].end_eid,
                        step.current.sp + 1,
                        LocationType::Stack,
                        true,
                        *args.get(0).unwrap(),
                    )?;

                    self.lookup_write_stack.assign(
                        ctx,
                        step.current.eid,
                        entry.memory_rw_entires[1].end_eid,
                        step.current.sp + 1,
                        LocationType::Stack,
                        false,
                        ret_val.unwrap(),
                    )?;
                } else {
                    let arg_type: VarType = (*signature.params.get(0).unwrap()).into();
                    assert_eq!(args.len(), 1);
                    assert_eq!(arg_type, VarType::I64);

                    let value = *args.first().unwrap();
                    self.value.assign(ctx, value)?;

                    self.is_wasm_output_op.assign_bool(ctx, true)?;

                    self.enable_input_table_lookup.assign_bool(ctx, true)?;
                    self.public_input_index_for_lookup
                        .assign(ctx, (step.current.host_public_inputs as u64).into())?;
                    self.value_for_lookup.assign(ctx, value.into())?;

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
                }

                Ok(())
            }

            _ => unreachable!(),
        }
    }

    fn mops(&self, meta: &mut VirtualCells<'_, F>) -> Option<Expression<F>> {
        Some(self.is_wasm_input_op.expr(meta))
    }

    fn memory_writing_ops(&self, entry: &EventTableEntry) -> u32 {
        match &entry.step_info {
            StepInfo::CallHost {
                plugin,
                op_index_in_plugin,
                ..
            } => {
                assert_eq!(*plugin, HostPlugin::HostInput);

                if *op_index_in_plugin == Op::WasmInput as usize {
                    1
                } else {
                    0
                }
            }
            _ => unreachable!(),
        }
    }

    fn input_index_increase(
        &self,
        meta: &mut VirtualCells<'_, F>,
        _common_config: &EventTableCommonConfig<F>,
    ) -> Option<Expression<F>> {
        // Public wasm input or wasm output
        Some(self.enable_input_table_lookup.expr(meta))
    }

    fn is_host_public_input(&self, entry: &EventTableEntry) -> bool {
        match &entry.step_info {
            StepInfo::CallHost {
                plugin,
                args,
                op_index_in_plugin,
                ..
            } => {
                assert_eq!(*plugin, HostPlugin::HostInput);

                *op_index_in_plugin == Op::WasmInput as usize && args[0] != 0
                    || *op_index_in_plugin == Op::WasmOutput as usize
            }
            _ => unreachable!(),
        }
    }
}
