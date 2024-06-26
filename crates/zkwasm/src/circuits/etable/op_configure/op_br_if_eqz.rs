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
use specs::itable::OPCODE_ARG0_SHIFT;
use specs::itable::OPCODE_ARG1_SHIFT;
use specs::itable::OPCODE_CLASS_SHIFT;
use specs::mtable::LocationType;
use specs::mtable::VarType;
use specs::step::StepInfo;

pub struct BrIfEqzConfig<F: FieldExt> {
    cond_inv_cell: AllocatedUnlimitedCell<F>,
    cond_is_zero_cell: AllocatedBitCell<F>,
    cond_is_not_zero_cell: AllocatedBitCell<F>,

    keep_cell: AllocatedBitCell<F>,
    is_i32_cell: AllocatedBitCell<F>,
    drop_cell: AllocatedCommonRangeCell<F>,
    dst_pc_cell: AllocatedCommonRangeCell<F>,
    memory_table_lookup_stack_read_cond: AllocatedMemoryTableLookupReadCell<F>,
    memory_table_lookup_stack_read_return_value: AllocatedMemoryTableLookupReadCell<F>,
    memory_table_lookup_stack_write_return_value: AllocatedMemoryTableLookupWriteCell<F>,
}

pub struct BrIfEqzConfigBuilder;

impl<F: FieldExt> EventTableOpcodeConfigBuilder<F> for BrIfEqzConfigBuilder {
    fn configure(
        common_config: &EventTableCommonConfig<F>,
        allocator: &mut EventTableCellAllocator<F>,
        constraint_builder: &mut ConstraintBuilder<F>,
    ) -> Box<dyn EventTableOpcodeConfig<F>> {
        let cond_inv_cell = allocator.alloc_unlimited_cell();
        let cond_is_zero_cell = allocator.alloc_bit_cell();
        let cond_is_not_zero_cell = allocator.alloc_bit_cell();

        let keep_cell = allocator.alloc_bit_cell();
        let is_i32_cell = allocator.alloc_bit_cell();
        let drop_cell = allocator.alloc_common_range_cell();
        let dst_pc_cell = allocator.alloc_common_range_cell();

        let eid = common_config.eid_cell;
        let sp = common_config.sp_cell;

        let memory_table_lookup_stack_read_cond = allocator
            .alloc_memory_table_lookup_read_cell_with_value(
                "op_br_if stack read cond",
                constraint_builder,
                eid,
                move |____| constant_from!(LocationType::Stack as u64),
                move |meta| sp.expr(meta) + constant_from!(1),
                move |____| constant_from!(1),
                move |____| constant_from!(1),
            );
        let cond_cell = memory_table_lookup_stack_read_cond.value_cell;

        let memory_table_lookup_stack_read_return_value = allocator
            .alloc_memory_table_lookup_read_cell_with_value(
                "op_br_if_eqz stack read return value",
                constraint_builder,
                eid,
                move |____| constant_from!(LocationType::Stack as u64),
                move |meta| sp.expr(meta) + constant_from!(2),
                move |meta| is_i32_cell.expr(meta),
                move |meta| keep_cell.expr(meta) * cond_is_zero_cell.expr(meta),
            );
        let value_cell = memory_table_lookup_stack_read_return_value.value_cell;

        let memory_table_lookup_stack_write_return_value = allocator
            .alloc_memory_table_lookup_write_cell(
                "op_br_if_eqz stack write return value",
                constraint_builder,
                eid,
                move |____| constant_from!(LocationType::Stack as u64),
                move |meta| sp.expr(meta) + drop_cell.expr(meta) + constant_from!(2),
                move |meta| is_i32_cell.expr(meta),
                move |meta| value_cell.expr(meta),
                move |meta| keep_cell.expr(meta) * cond_is_zero_cell.expr(meta),
            );

        constraint_builder.constraints.push((
            "op_br_if cond bit",
            Box::new(move |meta| {
                vec![
                    cond_is_zero_cell.expr(meta) * cond_cell.expr(meta),
                    cond_is_zero_cell.expr(meta) + cond_cell.expr(meta) * cond_inv_cell.expr(meta)
                        - constant_from!(1),
                    cond_is_zero_cell.expr(meta) + cond_is_not_zero_cell.expr(meta)
                        - constant_from!(1),
                ]
            }),
        ));

        Box::new(BrIfEqzConfig {
            cond_inv_cell,
            cond_is_zero_cell,
            cond_is_not_zero_cell,
            keep_cell,
            is_i32_cell,
            drop_cell,
            dst_pc_cell,
            memory_table_lookup_stack_read_cond,
            memory_table_lookup_stack_read_return_value,
            memory_table_lookup_stack_write_return_value,
        })
    }
}

impl<F: FieldExt> EventTableOpcodeConfig<F> for BrIfEqzConfig<F> {
    fn opcode(&self, meta: &mut VirtualCells<'_, F>) -> Expression<F> {
        constant!(bn_to_field(
            &(BigUint::from(OpcodeClass::BrIfEqz as u64) << OPCODE_CLASS_SHIFT)
        )) + self.drop_cell.expr(meta)
            * constant!(bn_to_field(&(BigUint::from(1u64) << OPCODE_ARG0_SHIFT)))
            + self.keep_cell.expr(meta)
                * constant!(bn_to_field(&(BigUint::from(1u64) << OPCODE_ARG1_SHIFT)))
            + self.dst_pc_cell.expr(meta)
    }

    fn assign(
        &self,
        ctx: &mut Context<'_, F>,
        step: &mut StepStatus<F>,
        entry: &EventTableEntryWithMemoryInfo,
    ) -> Result<(), Error> {
        match &entry.eentry.step_info {
            StepInfo::BrIfEqz {
                condition,
                dst_pc,
                drop,
                keep,
                keep_values,
            } => {
                assert!(keep.len() <= 1);

                let cond = *condition as u32 as u64;

                self.memory_table_lookup_stack_read_cond.assign(
                    ctx,
                    entry.memory_rw_entires[0].start_eid,
                    step.current.eid,
                    entry.memory_rw_entires[0].end_eid,
                    step.current.sp + 1,
                    LocationType::Stack,
                    true,
                    cond,
                )?;

                self.drop_cell.assign(ctx, F::from(*drop as u64))?;

                if !keep.is_empty() {
                    let keep_type: VarType = keep[0].into();

                    self.keep_cell.assign(ctx, F::one())?;
                    self.is_i32_cell.assign(ctx, F::from(keep_type as u64))?;
                    if *condition == 0 {
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
                            step.current.sp + *drop + 2,
                            LocationType::Stack,
                            VarType::from(keep[0]) == VarType::I32,
                            keep_values[0],
                        )?;
                    }
                }

                if cond != 0 {
                    self.cond_inv_cell
                        .assign(ctx, step.field_helper.invert(cond))?;
                }
                self.cond_is_zero_cell
                    .assign(ctx, if cond == 0 { F::one() } else { F::zero() })?;
                self.cond_is_not_zero_cell
                    .assign(ctx, if cond == 0 { F::zero() } else { F::one() })?;
                self.dst_pc_cell.assign(ctx, F::from((*dst_pc) as u64))?;
            }
            _ => unreachable!(),
        }

        Ok(())
    }

    fn sp_diff(&self, meta: &mut VirtualCells<'_, F>) -> Option<Expression<F>> {
        Some(constant_from!(1) + self.cond_is_zero_cell.expr(meta) * self.drop_cell.expr(meta))
    }

    fn mops(&self, meta: &mut VirtualCells<'_, F>) -> Option<Expression<F>> {
        Some(self.cond_is_zero_cell.expr(meta) * self.keep_cell.expr(meta))
    }

    fn memory_writing_ops(&self, entry: &EventTableEntry) -> u32 {
        match &entry.step_info {
            StepInfo::BrIfEqz {
                keep, condition, ..
            } => {
                if *condition != 0 {
                    0
                } else {
                    keep.len() as u32
                }
            }
            _ => unreachable!(),
        }
    }

    fn next_iid(
        &self,
        meta: &mut VirtualCells<'_, F>,
        common_config: &EventTableCommonConfig<F>,
    ) -> Option<Expression<F>> {
        Some(
            self.cond_is_zero_cell.expr(meta) * self.dst_pc_cell.expr(meta)
                + self.cond_is_not_zero_cell.expr(meta)
                    * (common_config.iid_cell.curr_expr(meta) + constant_from!(1)),
        )
    }
}
