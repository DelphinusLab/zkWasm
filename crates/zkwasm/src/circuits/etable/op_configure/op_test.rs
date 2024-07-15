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
use specs::encode::opcode::encode_test;
use specs::encode::opcode::UniArgEncode;
use specs::etable::EventTableEntry;
use specs::itable::TestOp;
use specs::mtable::LocationType;
use specs::mtable::VarType;
use specs::step::StepInfo;

pub struct TestConfig<F: FieldExt> {
    res_cell: AllocatedBitCell<F>,
    value_inv_cell: AllocatedUnlimitedCell<F>,

    operand_arg: EventTableCommonArgsConfig<F>,
    memory_table_lookup_stack_write: AllocatedMemoryTableLookupWriteCell<F>,
}

pub struct TestConfigBuilder {}

impl<F: FieldExt> EventTableOpcodeConfigBuilder<F> for TestConfigBuilder {
    fn configure(
        common_config: &EventTableCommonConfig<F>,
        allocator: &mut EventTableCellAllocator<F>,
        constraint_builder: &mut ConstraintBuilder<F>,
    ) -> Box<dyn EventTableOpcodeConfig<F>> {
        let res_cell = allocator.alloc_bit_cell();
        let value_inv_cell = allocator.alloc_unlimited_cell();

        let eid = common_config.eid_cell;
        let sp = common_config.sp_cell;

        let operand_arg = common_config.uniarg_configs[0];
        let value_cell = operand_arg.value_cell;
        constraint_builder.push(
            "op_test: uniarg",
            Box::new(move |meta| {
                vec![
                    common_config.uniarg_configs[0].is_enabled_cell.expr(meta) - constant_from!(1),
                    common_config
                        .uniarg_configs
                        .iter()
                        .skip(1)
                        .map(|x| x.is_enabled_cell.expr(meta))
                        .reduce(|l, r| l + r)
                        .unwrap(),
                ]
            }),
        );

        constraint_builder.constraints.push((
            "op_test res = !value",
            Box::new(move |meta| {
                vec![
                    res_cell.expr(meta) * value_cell.expr(meta),
                    value_cell.expr(meta) * value_inv_cell.expr(meta) - constant_from!(1)
                        + res_cell.expr(meta),
                ]
            }),
        ));

        let memory_table_lookup_stack_write = allocator.alloc_memory_table_lookup_write_cell(
            "op_test stack write",
            constraint_builder,
            eid,
            move |____| constant_from!(LocationType::Stack as u64),
            move |meta| sp.expr(meta) + constant_from!(1),
            move |____| constant_from!(1),
            move |meta| res_cell.expr(meta),
            move |____| constant_from!(1),
        );

        Box::new(TestConfig {
            res_cell,
            value_inv_cell,
            operand_arg,
            memory_table_lookup_stack_write,
        })
    }
}

impl<F: FieldExt> EventTableOpcodeConfig<F> for TestConfig<F> {
    fn opcode(&self, meta: &mut VirtualCells<'_, F>) -> Expression<F> {
        encode_test(
            constant_from!(TestOp::Eqz as u16),
            self.operand_arg.is_i32_cell.expr(meta),
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
            StepInfo::Test {
                vtype,
                value,
                result,
            } => {
                if *value != 0 {
                    self.value_inv_cell
                        .assign(ctx, step.field_helper.invert(*value))?;
                }
                self.res_cell.assign_u32(ctx, *result as u32)?;

                todo!();
                //self.operand_arg.assign()

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
}
