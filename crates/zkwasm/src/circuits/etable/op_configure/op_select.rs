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
use specs::encode::opcode::encode_select;
use specs::encode::opcode::UniArgEncode;
use specs::etable::EventTableEntry;
use specs::mtable::LocationType;
use specs::step::StepInfo;

pub struct SelectConfig<F: FieldExt> {
    cond_inv: AllocatedUnlimitedCell<F>,
    res: AllocatedUnlimitedCell<F>,

    cond_arg: EventTableCommonArgsConfig<F>,
    rhs_arg: EventTableCommonArgsConfig<F>,
    lhs_arg: EventTableCommonArgsConfig<F>,
    memory_table_lookup_stack_write: AllocatedMemoryTableLookupWriteCell<F>,
}

pub struct SelectConfigBuilder {}

impl<F: FieldExt> EventTableOpcodeConfigBuilder<F> for SelectConfigBuilder {
    fn configure(
        common_config: &EventTableCommonConfig<F>,
        allocator: &mut EventTableCellAllocator<F>,
        constraint_builder: &mut ConstraintBuilder<F>,
    ) -> Box<dyn EventTableOpcodeConfig<F>> {
        let cond_inv = allocator.alloc_unlimited_cell();
        let res = allocator.alloc_unlimited_cell();

        let eid = common_config.eid_cell;
        let sp = common_config.sp_cell;

        let cond_arg = common_config.uniarg_configs[0].clone();
        let rhs_arg = common_config.uniarg_configs[1].clone();
        let lhs_arg = common_config.uniarg_configs[2].clone();
        let is_i32 = lhs_arg.is_i32_cell;
        let cond = cond_arg.value_cell;
        let lhs = lhs_arg.value_cell;
        let rhs = rhs_arg.value_cell;
        constraint_builder.push(
            "select: uniarg",
            Box::new(move |meta| {
                vec![
                    cond_arg.is_i32_cell.expr(meta) - constant_from!(1),
                    rhs_arg.is_i32_cell.expr(meta) - lhs_arg.is_i32_cell.expr(meta),
                ]
            }),
        );

        constraint_builder.push(
            "select: cond is zero",
            Box::new(move |meta| {
                vec![
                    (constant_from!(1) - cond.expr(meta) * cond_inv.expr(meta))
                        * (res.expr(meta) - rhs.expr(meta)),
                ]
            }),
        );

        constraint_builder.push(
            "select: cond is not zero",
            Box::new(move |meta| vec![cond.expr(meta) * (res.expr(meta) - lhs.expr(meta))]),
        );

        let uniarg_configs = common_config.uniarg_configs.clone();
        let memory_table_lookup_stack_write = allocator.alloc_memory_table_lookup_write_cell(
            "op_select stack write",
            constraint_builder,
            eid,
            move |____| constant_from!(LocationType::Stack as u64),
            move |meta| Self::sp_after_uniarg(sp, &uniarg_configs, meta),
            move |meta| is_i32.expr(meta),
            move |meta| res.expr(meta),
            move |____| constant_from!(1),
        );

        Box::new(SelectConfig {
            cond_inv,
            res,
            cond_arg,
            rhs_arg,
            lhs_arg,
            memory_table_lookup_stack_write,
        })
    }
}

impl<F: FieldExt> EventTableOpcodeConfig<F> for SelectConfig<F> {
    fn opcode(&self, _: &mut VirtualCells<'_, F>) -> Expression<F> {
        encode_select(UniArgEncode::Reserve)
    }

    fn assign(
        &self,
        ctx: &mut Context<'_, F>,
        step: &mut StepStatus<F>,
        entry: &EventTableEntryWithMemoryInfo,
    ) -> Result<(), Error> {
        match &entry.eentry.step_info {
            StepInfo::Select { cond, result, .. } => {
                if *cond != 0 {
                    self.cond_inv.assign(ctx, step.field_helper.invert(*cond))?;
                }
                self.res.assign(ctx, F::from(*result))?;

                if let specs::itable::Opcode::Select { uniargs, .. } =
                    entry.eentry.get_instruction(step.current.itable).opcode
                {
                    let mut memory_entries = entry.memory_rw_entires.iter();

                    self.cond_arg
                        .assign(ctx, &uniargs[0], &mut memory_entries)?;
                    self.rhs_arg.assign(ctx, &uniargs[1], &mut memory_entries)?;
                    self.lhs_arg.assign(ctx, &uniargs[2], &mut memory_entries)?;
                    self.memory_table_lookup_stack_write
                        .assign_with_memory_entry(ctx, &mut memory_entries)?;
                } else {
                    unreachable!();
                }

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

    fn memory_writing_ops(&self, _: &EventTableEntry) -> u32 {
        1
    }
}
