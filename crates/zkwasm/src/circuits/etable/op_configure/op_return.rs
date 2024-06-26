use crate::circuits::cell::*;
use crate::circuits::etable::allocator::*;
use crate::circuits::etable::ConstraintBuilder;
use crate::circuits::etable::EventTableCommonConfig;
use crate::circuits::etable::EventTableOpcodeConfig;
use crate::circuits::etable::EventTableOpcodeConfigBuilder;
use crate::circuits::jtable::expression::JtableLookupEntryEncode;
use crate::circuits::jtable::JumpTableConfig;
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
use num_bigint::ToBigUint;
use specs::encode::frame_table::encode_frame_table_entry;
use specs::etable::EventTableEntry;
use specs::itable::OpcodeClass;
use specs::itable::OPCODE_ARG0_SHIFT;
use specs::itable::OPCODE_ARG1_SHIFT;
use specs::itable::OPCODE_CLASS_SHIFT;
use specs::mtable::LocationType;
use specs::mtable::VarType;
use specs::step::StepInfo;

pub struct ReturnConfig<F: FieldExt> {
    keep: AllocatedBitCell<F>,
    drop: AllocatedCommonRangeCell<F>,
    is_i32: AllocatedBitCell<F>,
    value: AllocatedU64Cell<F>,
    // always assign to one to support sliced frame table lookup
    is_returned_cell: AllocatedBitCell<F>,
    frame_table_lookup: AllocatedUnlimitedCell<F>,
    memory_table_lookup_stack_read: AllocatedMemoryTableLookupReadCell<F>,
    memory_table_lookup_stack_write: AllocatedMemoryTableLookupWriteCell<F>,
}

pub struct ReturnConfigBuilder;

impl<F: FieldExt> EventTableOpcodeConfigBuilder<F> for ReturnConfigBuilder {
    fn configure(
        common_config: &EventTableCommonConfig<F>,
        allocator: &mut EventTableCellAllocator<F>,
        constraint_builder: &mut ConstraintBuilder<F>,
    ) -> Box<dyn EventTableOpcodeConfig<F>> {
        let keep = allocator.alloc_bit_cell();
        let drop = allocator.alloc_common_range_cell();
        let is_i32 = allocator.alloc_bit_cell();
        let value = allocator.alloc_u64_cell();

        let frame_table_lookup = common_config.jtable_lookup_cell;

        let fid_cell = common_config.fid_cell;
        let iid_cell = common_config.iid_cell;
        let frame_id_cell = common_config.frame_id_cell;
        let eid = common_config.eid_cell;
        let sp = common_config.sp_cell;
        let is_returned_cell = common_config.is_returned_cell;

        let memory_table_lookup_stack_read = allocator.alloc_memory_table_lookup_read_cell(
            "op_return stack read",
            constraint_builder,
            eid,
            move |____| constant_from!(LocationType::Stack as u64),
            move |meta| sp.expr(meta) + constant_from!(1),
            move |meta| is_i32.expr(meta),
            move |meta| value.u64_cell.expr(meta),
            move |meta| keep.expr(meta),
        );
        let memory_table_lookup_stack_write = allocator.alloc_memory_table_lookup_write_cell(
            "op_return stack write",
            constraint_builder,
            eid,
            move |____| constant_from!(LocationType::Stack as u64),
            move |meta| sp.expr(meta) + drop.expr(meta) + constant_from!(1),
            move |meta| is_i32.expr(meta),
            move |meta| value.u64_cell.expr(meta),
            move |meta| keep.expr(meta),
        );

        constraint_builder.constraints.push((
            "return frame table lookups",
            Box::new(move |meta| {
                vec![
                    frame_table_lookup.cell.expr(meta)
                        - JumpTableConfig::encode_lookup(
                            frame_id_cell.expr(meta),
                            frame_id_cell.next_expr(meta),
                            fid_cell.expr(meta),
                            fid_cell.next_expr(meta),
                            iid_cell.next_expr(meta),
                        ),
                ]
            }),
        ));

        constraint_builder.constraints.push((
            "return: returned bit",
            Box::new(move |meta| vec![is_returned_cell.expr(meta) - constant_from!(1)]),
        ));

        Box::new(ReturnConfig {
            keep,
            drop,
            is_i32,
            value,
            frame_table_lookup,
            is_returned_cell,
            memory_table_lookup_stack_read,
            memory_table_lookup_stack_write,
        })
    }
}

impl<F: FieldExt> EventTableOpcodeConfig<F> for ReturnConfig<F> {
    fn opcode(&self, meta: &mut VirtualCells<'_, F>) -> Expression<F> {
        constant!(bn_to_field(
            &(BigUint::from(OpcodeClass::Return as u64) << OPCODE_CLASS_SHIFT)
        )) + self.drop.expr(meta)
            * constant!(bn_to_field(&(BigUint::from(1u64) << OPCODE_ARG0_SHIFT)))
            + self.keep.expr(meta)
                * constant!(bn_to_field(&(BigUint::from(1u64) << OPCODE_ARG1_SHIFT)))
            + self.is_i32.expr(meta)
    }

    fn assign(
        &self,
        ctx: &mut Context<'_, F>,
        step: &mut StepStatus<F>,
        entry: &EventTableEntryWithMemoryInfo,
    ) -> Result<(), Error> {
        match &entry.eentry.step_info {
            StepInfo::Return {
                drop,
                keep,
                keep_values,
                ..
            } => {
                assert!(keep.len() <= 1);
                assert_eq!(keep.len(), keep_values.len());

                self.drop.assign(ctx, F::from(*drop as u64))?;

                if keep_values.is_empty() {
                    self.keep.assign(ctx, 0.into())?;
                } else {
                    self.keep.assign(ctx, 1.into())?;
                    self.is_i32
                        .assign(ctx, (VarType::from(keep[0]) as u64).into())?;
                    self.value.assign(ctx, keep_values[0])?;

                    self.memory_table_lookup_stack_read.assign(
                        ctx,
                        entry.memory_rw_entires[0].start_eid,
                        step.current.eid,
                        entry.memory_rw_entires[0].end_eid,
                        step.current.sp + 1,
                        LocationType::Stack,
                        VarType::from(keep[0]) == VarType::I32,
                        keep_values[0],
                    )?;

                    self.memory_table_lookup_stack_write.assign(
                        ctx,
                        step.current.eid,
                        entry.memory_rw_entires[1].end_eid,
                        step.current.sp + drop + 1,
                        LocationType::Stack,
                        VarType::from(keep[0]) == VarType::I32,
                        keep_values[0],
                    )?;
                }

                self.frame_table_lookup.cell.assign_bn(
                    ctx,
                    &encode_frame_table_entry(
                        step.current.last_jump_eid.to_biguint().unwrap(),
                        step.next.last_jump_eid.to_biguint().unwrap(),
                        step.current.fid.to_biguint().unwrap(),
                        step.next.fid.to_biguint().unwrap(),
                        step.next.iid.to_biguint().unwrap(),
                    ),
                )?;

                self.is_returned_cell.assign(ctx, 1.into())?;

                Ok(())
            }
            _ => unreachable!(),
        }
    }

    fn sp_diff(&self, meta: &mut VirtualCells<'_, F>) -> Option<Expression<F>> {
        Some(self.drop.expr(meta))
    }

    fn mops(&self, meta: &mut VirtualCells<'_, F>) -> Option<Expression<F>> {
        Some(self.keep.expr(meta))
    }

    fn memory_writing_ops(&self, entry: &EventTableEntry) -> u32 {
        match &entry.step_info {
            StepInfo::Return { keep, .. } => {
                if !keep.is_empty() {
                    assert!(keep.len() == 1);
                    1
                } else {
                    0
                }
            }
            _ => unreachable!(),
        }
    }

    fn return_ops_expr(&self, _meta: &mut VirtualCells<'_, F>) -> Option<Expression<F>> {
        Some(constant_from!(self.return_ops() as u64))
    }

    fn return_ops(&self) -> u32 {
        1
    }

    fn next_frame_id(
        &self,
        meta: &mut VirtualCells<'_, F>,
        common_config: &EventTableCommonConfig<F>,
    ) -> Option<Expression<F>> {
        Some(common_config.frame_id_cell.next_expr(meta))
    }

    fn next_fid(
        &self,
        meta: &mut VirtualCells<'_, F>,
        common_config: &EventTableCommonConfig<F>,
    ) -> Option<Expression<F>> {
        Some(common_config.fid_cell.next_expr(meta))
    }

    fn next_iid(
        &self,
        meta: &mut VirtualCells<'_, F>,
        common_config: &EventTableCommonConfig<F>,
    ) -> Option<Expression<F>> {
        Some(common_config.iid_cell.next_expr(meta))
    }
}
