use crate::{
    circuits::{
        cell::*,
        etable_compact::op_configure::{BrTableLookupCell, MTableLookupCell},
        etable_v2::{
            allocator::*, ConstraintBuilder, EventTableCommonConfig, EventTableOpcodeConfig,
            EventTableOpcodeConfigBuilder,
        },
        jtable::{expression::JtableLookupEntryEncode, JumpTableConfig},
        mtable_compact::encode::MemoryTableLookupEncode,
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
    encode::{
        br_table::encode_elem_entry,
        frame_table::encode_frame_table_entry,
        opcode::{encode_call, encode_call_indirect},
    },
    etable::EventTableEntry,
    itable::{OpcodeClass, OPCODE_ARG0_SHIFT, OPCODE_CLASS_SHIFT},
    mtable::{LocationType, VarType},
    step::StepInfo,
};

pub struct CallIndirectConfig<F: FieldExt> {
    type_index: AllocatedCommonRangeCell<F>,
    func_index: AllocatedCommonRangeCell<F>,
    offset: AllocatedCommonRangeCell<F>,
    table_index: AllocatedCommonRangeCell<F>,

    memory_table_lookup_stack_read: AllocatedMemoryTableLookupReadCell<F>,
    elem_lookup: AllocatedUnlimitedCell<F>,
    frame_table_lookup: AllocatedUnlimitedCell<F>,
}

pub struct CallIndirectConfigBuilder {}

impl<F: FieldExt> EventTableOpcodeConfigBuilder<F> for CallIndirectConfigBuilder {
    fn configure(
        common_config: &EventTableCommonConfig<F>,
        allocator: &mut EventTableCellAllocator<F>,
        constraint_builder: &mut ConstraintBuilder<F>,
    ) -> Box<dyn EventTableOpcodeConfig<F>> {
        let type_index = allocator.alloc_common_range_cell();
        let table_index = allocator.alloc_common_range_cell();
        let offset = allocator.alloc_common_range_cell();
        let func_index = allocator.alloc_common_range_cell();

        // Wasmi only support one table.
        constraint_builder.push(
            "table_index",
            Box::new(move |meta| vec![table_index.expr(meta)]),
        );

        let elem_lookup = common_config.brtable_lookup_cell;

        constraint_builder.push(
            "op_call_indirect elem table lookup",
            Box::new(move |meta| {
                vec![
                    elem_lookup.expr(meta)
                        - encode_elem_entry(
                            table_index.expr(meta),
                            type_index.expr(meta),
                            offset.expr(meta),
                            func_index.expr(meta),
                        ),
                ]
            }),
        );

        let eid = common_config.eid_cell;
        let sp = common_config.sp_cell;

        let memory_table_lookup_stack_read = allocator.alloc_memory_table_lookup_read_cell(
            "op_call_indirect stack read",
            constraint_builder,
            eid,
            move |meta| constant_from!(LocationType::Stack),
            move |meta| sp.expr(meta) + constant_from!(1),
            move |meta| constant_from!(1),
            move |meta| offset.expr(meta),
            move |meta| constant_from!(1),
        );

        let fid_cell = common_config.fid_cell;
        let iid_cell = common_config.iid_cell;
        let frame_id_cell = common_config.frame_id_cell;
        let eid = common_config.eid_cell;
        let frame_table_lookup = common_config.jtable_lookup_cell;

        constraint_builder.constraints.push((
            "return frame table lookups",
            Box::new(move |meta| {
                vec![
                    frame_table_lookup.expr(meta)
                        - JumpTableConfig::encode_lookup(
                            eid.expr(meta),
                            frame_id_cell.expr(meta),
                            func_index.expr(meta),
                            fid_cell.expr(meta),
                            iid_cell.expr(meta) + constant_from!(1),
                        ),
                ]
            }),
        ));

        Box::new(CallIndirectConfig {
            type_index,
            func_index,
            offset,
            table_index,
            memory_table_lookup_stack_read,
            elem_lookup,
            frame_table_lookup,
        })
    }
}

impl<F: FieldExt> EventTableOpcodeConfig<F> for CallIndirectConfig<F> {
    fn opcode(&self, meta: &mut VirtualCells<'_, F>) -> Expression<F> {
        encode_call_indirect(self.type_index.expr(meta))
    }

    fn assign(
        &self,
        ctx: &mut Context<'_, F>,
        step: &StepStatus,
        entry: &EventTableEntryWithMemoryInfo,
    ) -> Result<(), Error> {
        match &entry.eentry.step_info {
            StepInfo::CallIndirect {
                table_index,
                type_index,
                offset,
                func_index,
            } => {
                self.table_index.assign(ctx, F::from(*table_index as u64))?;
                self.type_index.assign(ctx, F::from(*type_index as u64))?;
                self.offset.assign(ctx, F::from(*offset as u64))?;
                self.func_index.assign(ctx, F::from(*func_index as u64))?;

                self.elem_lookup.assign_bn(
                    ctx,
                    &encode_elem_entry(
                        BigUint::from(*table_index),
                        BigUint::from(*type_index),
                        BigUint::from(*offset),
                        BigUint::from(*func_index),
                    ),
                )?;

                self.memory_table_lookup_stack_read.assign(
                    ctx,
                    entry.memory_rw_entires[0].start_eid,
                    step.current.eid,
                    entry.memory_rw_entires[0].end_eid,
                    step.current.sp + 1,
                    LocationType::Stack,
                    true,
                    *offset as u64,
                )?;

                self.frame_table_lookup.assign_bn(
                    ctx,
                    &encode_frame_table_entry(
                        step.current.eid.into(),
                        step.current.last_jump_eid.into(),
                        (*func_index).into(),
                        step.current.fid.into(),
                        (step.current.iid + 1).into(),
                    ),
                )?;

                Ok(())
            }

            _ => unreachable!(),
        }
    }
    
    fn sp_diff(&self, _meta: &mut VirtualCells<'_, F>) -> Option<Expression<F>> {
        Some(constant!(F::one()))
    }

    fn jops_expr(&self, _meta: &mut VirtualCells<'_, F>) -> Option<Expression<F>> {
        Some(constant_from!(1))
    }

    fn jops(&self) -> u32 {
        1
    }

    fn next_frame_id(
        &self,
        meta: &mut VirtualCells<'_, F>,
        common_config: &EventTableCommonConfig<F>,
    ) -> Option<Expression<F>> {
        Some(common_config.eid_cell.curr_expr(meta))
    }

    fn next_fid(
        &self,
        meta: &mut VirtualCells<'_, F>,
        _common_config: &EventTableCommonConfig<F>,
    ) -> Option<Expression<F>> {
        Some(self.func_index.expr(meta))
    }

    fn next_iid(
        &self,
        _meta: &mut VirtualCells<'_, F>,
        _common_config: &EventTableCommonConfig<F>,
    ) -> Option<Expression<F>> {
        Some(constant_from!(0))
    }
}
