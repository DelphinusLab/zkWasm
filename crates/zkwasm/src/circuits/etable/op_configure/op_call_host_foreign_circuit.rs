use crate::circuits::cell::*;
use crate::circuits::etable::allocator::*;
use crate::circuits::etable::ConstraintBuilder;
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
use num_bigint::BigUint;
use specs::encode::opcode::encode_call_host;
use specs::external_host_call_table::encode::encode_host_call_entry;
use specs::external_host_call_table::ExternalHostCallSignature;
use specs::mtable::LocationType;
use specs::step::StepInfo;

pub struct ExternalCallHostCircuitConfig<F: FieldExt> {
    op: AllocatedCommonRangeCell<F>,
    value_is_ret: AllocatedBitCell<F>,
    value_is_not_ret: AllocatedBitCell<F>,

    external_foreign_call_lookup_cell: AllocatedUnlimitedCell<F>,
    memory_table_lookup_stack_read: AllocatedMemoryTableLookupReadCell<F>,
    memory_table_lookup_stack_write: AllocatedMemoryTableLookupWriteCell<F>,
}

pub struct ExternalCallHostCircuitConfigBuilder {}

impl<F: FieldExt> EventTableOpcodeConfigBuilder<F> for ExternalCallHostCircuitConfigBuilder {
    fn configure(
        common_config: &EventTableCommonConfig<F>,
        allocator: &mut EventTableCellAllocator<F>,
        constraint_builder: &mut ConstraintBuilder<F>,
    ) -> Box<dyn EventTableOpcodeConfig<F>> {
        let op = allocator.alloc_common_range_cell();
        let value_is_ret = allocator.alloc_bit_cell();
        let value_is_not_ret = allocator.alloc_bit_cell();

        let index = common_config.external_host_call_index_cell;
        let external_foreign_call_lookup_cell = common_config.external_foreign_call_lookup_cell;

        constraint_builder.push(
            "op_call_host is_ret or not",
            Box::new(move |meta| {
                vec![value_is_ret.expr(meta) + value_is_not_ret.expr(meta) - constant_from!(1)]
            }),
        );

        constraint_builder.push(
            "external host call index change",
            Box::new(move |meta| {
                vec![index.next_expr(meta) - index.curr_expr(meta) - constant_from!(1)]
            }),
        );

        let eid = common_config.eid_cell;
        let sp = common_config.sp_cell;

        let memory_table_lookup_stack_read = allocator
            .alloc_memory_table_lookup_read_cell_with_value(
                "op_call_host read value",
                constraint_builder,
                eid,
                move |____| constant_from!(LocationType::Stack),
                move |meta| sp.expr(meta) + constant_from!(1),
                move |____| constant_from!(0),
                move |meta| value_is_not_ret.expr(meta),
            );

        let memory_table_lookup_stack_write = allocator
            .alloc_memory_table_lookup_write_cell_with_value(
                "op_call_host return value",
                constraint_builder,
                eid,
                move |____| constant_from!(LocationType::Stack),
                move |meta| sp.expr(meta),
                move |____| constant_from!(0),
                move |meta| value_is_ret.expr(meta),
            );

        constraint_builder.push(
            "external host call lookup",
            Box::new(move |meta| {
                let operand = value_is_not_ret.expr(meta)
                    * memory_table_lookup_stack_read.value_cell.expr(meta)
                    + value_is_ret.expr(meta)
                        * memory_table_lookup_stack_write.value_cell.expr(meta);

                vec![
                    external_foreign_call_lookup_cell.expr(meta)
                        - encode_host_call_entry(index.expr(meta), op.expr(meta), operand),
                ]
            }),
        );

        Box::new(ExternalCallHostCircuitConfig {
            op,
            value_is_ret,
            value_is_not_ret,
            external_foreign_call_lookup_cell,
            memory_table_lookup_stack_read,
            memory_table_lookup_stack_write,
        })
    }
}

impl<F: FieldExt> EventTableOpcodeConfig<F> for ExternalCallHostCircuitConfig<F> {
    fn opcode(&self, meta: &mut VirtualCells<'_, F>) -> Expression<F> {
        encode_call_host(self.op.expr(meta), self.value_is_ret.expr(meta))
    }

    fn assign(
        &self,
        ctx: &mut Context<'_, F>,
        step: &mut StepStatus<F>,
        entry: &EventTableEntryWithMemoryInfo,
    ) -> Result<(), Error> {
        match &entry.eentry.step_info {
            StepInfo::ExternalHostCall { op, value, sig } => {
                self.op.assign(ctx, F::from(*op as u64))?;
                self.value_is_ret.assign_bool(ctx, sig.is_ret())?;
                self.value_is_not_ret.assign_bool(ctx, !sig.is_ret())?;
                self.external_foreign_call_lookup_cell.assign_bn(
                    ctx,
                    &encode_host_call_entry(
                        BigUint::from(step.current.external_host_call_call_index),
                        BigUint::from(*op as u64),
                        BigUint::from(value.unwrap()),
                    ),
                )?;

                match sig {
                    ExternalHostCallSignature::Argument => {
                        self.memory_table_lookup_stack_read.assign(
                            ctx,
                            entry.memory_rw_entires[0].start_eid,
                            step.current.eid,
                            entry.memory_rw_entires[0].end_eid,
                            step.current.sp + 1,
                            LocationType::Stack,
                            false,
                            value.unwrap(),
                        )?;
                    }
                    ExternalHostCallSignature::Return => {
                        self.memory_table_lookup_stack_write.assign(
                            ctx,
                            step.current.eid,
                            entry.memory_rw_entires[0].end_eid,
                            step.current.sp,
                            LocationType::Stack,
                            false,
                            value.unwrap(),
                        )?;
                    }
                }

                Ok(())
            }

            _ => unreachable!(),
        }
    }

    fn sp_diff(&self, meta: &mut VirtualCells<'_, F>) -> Option<Expression<F>> {
        Some(constant_from!(1) - self.value_is_ret.expr(meta) * constant_from!(2))
    }

    fn memory_writing_ops(&self, entry: &specs::etable::EventTableEntry) -> u32 {
        match &entry.step_info {
            StepInfo::ExternalHostCall { sig, .. } => match sig {
                ExternalHostCallSignature::Return => 1u32,
                _ => 0,
            },
            _ => unreachable!(),
        }
    }

    fn mops(&self, meta: &mut VirtualCells<'_, F>) -> Option<Expression<F>> {
        Some(self.value_is_ret.curr_expr(meta))
    }

    fn is_external_host_call(&self, _entry: &specs::etable::EventTableEntry) -> bool {
        true
    }

    fn external_host_call_index_increase(
        &self,
        _meta: &mut VirtualCells<'_, F>,
        _common_config: &EventTableCommonConfig<F>,
    ) -> Option<Expression<F>> {
        Some(constant_from!(1))
    }
}
