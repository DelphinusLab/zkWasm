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
use specs::mtable::VarType;
use specs::step::StepInfo;

pub struct SelectConfig<F: FieldExt> {
    cond: AllocatedU64Cell<F>,
    cond_inv: AllocatedUnlimitedCell<F>,

    val1: AllocatedU64Cell<F>,
    val2: AllocatedU64Cell<F>,
    res: AllocatedU64Cell<F>,
    is_i32: AllocatedBitCell<F>,

    memory_table_lookup_stack_read_cond: AllocatedMemoryTableLookupReadCell<F>,
    memory_table_lookup_stack_read_val2: AllocatedMemoryTableLookupReadCell<F>,
    memory_table_lookup_stack_read_val1: AllocatedMemoryTableLookupReadCell<F>,
    memory_table_lookup_stack_write: AllocatedMemoryTableLookupWriteCell<F>,
}

pub struct SelectConfigBuilder {}

impl<F: FieldExt> EventTableOpcodeConfigBuilder<F> for SelectConfigBuilder {
    fn configure(
        common_config: &EventTableCommonConfig<F>,
        allocator: &mut EventTableCellAllocator<F>,
        constraint_builder: &mut ConstraintBuilder<F>,
    ) -> Box<dyn EventTableOpcodeConfig<F>> {
        let cond = allocator.alloc_u64_cell();
        let cond_inv = allocator.alloc_unlimited_cell();

        let val1 = allocator.alloc_u64_cell();
        let val2 = allocator.alloc_u64_cell();
        let res = allocator.alloc_u64_cell();
        let is_i32 = allocator.alloc_bit_cell();

        constraint_builder.push(
            "select: cond is zero",
            Box::new(move |meta| {
                vec![
                    (constant_from!(1) - cond.u64_cell.expr(meta) * cond_inv.expr(meta))
                        * (res.u64_cell.expr(meta) - val2.u64_cell.expr(meta)),
                ]
            }),
        );

        constraint_builder.push(
            "select: cond is not zero",
            Box::new(move |meta| {
                vec![
                    cond.u64_cell.expr(meta) * (res.u64_cell.expr(meta) - val1.u64_cell.expr(meta)),
                ]
            }),
        );

        let eid = common_config.eid_cell;
        let sp = common_config.sp_cell;

        let memory_table_lookup_stack_read_cond = allocator.alloc_memory_table_lookup_read_cell(
            "op_select stack read",
            constraint_builder,
            eid,
            move |____| constant_from!(LocationType::Stack as u64),
            move |meta| sp.expr(meta) + constant_from!(1),
            move |____| constant_from!(1),
            move |meta| cond.u64_cell.expr(meta),
            move |____| constant_from!(1),
        );

        let memory_table_lookup_stack_read_val2 = allocator.alloc_memory_table_lookup_read_cell(
            "op_select stack read",
            constraint_builder,
            eid,
            move |____| constant_from!(LocationType::Stack as u64),
            move |meta| sp.expr(meta) + constant_from!(2),
            move |meta| is_i32.expr(meta),
            move |meta| val2.u64_cell.expr(meta),
            move |____| constant_from!(1),
        );

        let memory_table_lookup_stack_read_val1 = allocator.alloc_memory_table_lookup_read_cell(
            "op_select stack read",
            constraint_builder,
            eid,
            move |____| constant_from!(LocationType::Stack as u64),
            move |meta| sp.expr(meta) + constant_from!(3),
            move |meta| is_i32.expr(meta),
            move |meta| val1.u64_cell.expr(meta),
            move |____| constant_from!(1),
        );

        let memory_table_lookup_stack_write = allocator.alloc_memory_table_lookup_write_cell(
            "op_select stack write",
            constraint_builder,
            eid,
            move |____| constant_from!(LocationType::Stack as u64),
            move |meta| sp.expr(meta) + constant_from!(3),
            move |meta| is_i32.expr(meta),
            move |meta| res.u64_cell.expr(meta),
            move |____| constant_from!(1),
        );

        Box::new(SelectConfig {
            cond,
            cond_inv,
            val1,
            val2,
            res,
            is_i32,
            memory_table_lookup_stack_read_cond,
            memory_table_lookup_stack_read_val2,
            memory_table_lookup_stack_read_val1,
            memory_table_lookup_stack_write,
        })
    }
}

impl<F: FieldExt> EventTableOpcodeConfig<F> for SelectConfig<F> {
    fn opcode(&self, _: &mut VirtualCells<'_, F>) -> Expression<F> {
        constant!(bn_to_field(
            &(BigUint::from(OpcodeClass::Select as u64) << OPCODE_CLASS_SHIFT)
        ))
    }

    fn assign(
        &self,
        ctx: &mut Context<'_, F>,
        step: &mut StepStatus<F>,
        entry: &EventTableEntryWithMemoryInfo,
    ) -> Result<(), Error> {
        match &entry.eentry.step_info {
            StepInfo::Select {
                val1,
                val2,
                cond,
                result,
                vtype,
            } => {
                self.val1.assign(ctx, *val1)?;
                self.val2.assign(ctx, *val2)?;
                self.cond.assign(ctx, *cond)?;
                if *cond != 0 {
                    self.cond_inv.assign(ctx, step.field_helper.invert(*cond))?;
                }
                self.res.assign(ctx, *result)?;
                self.is_i32.assign_bool(ctx, *vtype == VarType::I32)?;

                self.memory_table_lookup_stack_read_cond.assign(
                    ctx,
                    entry.memory_rw_entires[0].start_eid,
                    step.current.eid,
                    entry.memory_rw_entires[0].end_eid,
                    step.current.sp + 1,
                    LocationType::Stack,
                    true,
                    *cond,
                )?;

                self.memory_table_lookup_stack_read_val2.assign(
                    ctx,
                    entry.memory_rw_entires[1].start_eid,
                    step.current.eid,
                    entry.memory_rw_entires[1].end_eid,
                    step.current.sp + 2,
                    LocationType::Stack,
                    *vtype == VarType::I32,
                    *val2,
                )?;

                self.memory_table_lookup_stack_read_val1.assign(
                    ctx,
                    entry.memory_rw_entires[2].start_eid,
                    step.current.eid,
                    entry.memory_rw_entires[2].end_eid,
                    step.current.sp + 3,
                    LocationType::Stack,
                    *vtype == VarType::I32,
                    *val1,
                )?;

                self.memory_table_lookup_stack_write.assign(
                    ctx,
                    step.current.eid,
                    entry.memory_rw_entires[3].end_eid,
                    step.current.sp + 3,
                    LocationType::Stack,
                    *vtype == VarType::I32,
                    *result,
                )?;

                Ok(())
            }

            _ => unreachable!(),
        }
    }

    fn sp_diff(&self, _meta: &mut VirtualCells<'_, F>) -> Option<Expression<F>> {
        Some(constant_from!(2))
    }

    fn mops(&self, _meta: &mut VirtualCells<'_, F>) -> Option<Expression<F>> {
        Some(constant_from!(1))
    }

    fn memory_writing_ops(&self, _: &EventTableEntry) -> u32 {
        1
    }
}
