use crate::{
    circuits::{
        cell::*,
        etable_v2::{
            allocator::*, ConstraintBuilder, EventTableCommonConfig, EventTableOpcodeConfig,
            EventTableOpcodeConfigBuilder,
        },
        jtable::{expression::JtableLookupEntryEncode, JumpTableConfig},
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
    encode::{frame_table::encode_frame_table_entry, opcode::encode_call},
    etable::EventTableEntry,
    itable::{OpcodeClass, TestOp, OPCODE_ARG0_SHIFT, OPCODE_ARG1_SHIFT, OPCODE_CLASS_SHIFT},
    mtable::{LocationType, VarType},
    step::StepInfo,
};

pub struct TestConfig<F: FieldExt> {
    is_i32_cell: AllocatedBitCell<F>,
    res_cell: AllocatedBitCell<F>,
    value_inv_cell: AllocatedUnlimitedCell<F>,
    value_cell: AllocatedU64Cell<F>,

    memory_table_lookup_stack_read: AllocatedMemoryTableLookupReadCell<F>,
    memory_table_lookup_stack_write: AllocatedMemoryTableLookupWriteCell<F>,
}

pub struct TestConfigBuilder {}

impl<F: FieldExt> EventTableOpcodeConfigBuilder<F> for TestConfigBuilder {
    fn configure(
        common_config: &EventTableCommonConfig<F>,
        allocator: &mut EventTableCellAllocator<F>,
        constraint_builder: &mut ConstraintBuilder<F>,
    ) -> Box<dyn EventTableOpcodeConfig<F>> {
        let is_i32_cell = allocator.alloc_bit_cell();
        let res_cell = allocator.alloc_bit_cell();
        let value_cell = allocator.alloc_u64_cell();
        let value_inv_cell = allocator.alloc_unlimited_cell();

        let eid = common_config.eid_cell;
        let sp = common_config.sp_cell;

        let memory_table_lookup_stack_read = allocator.alloc_memory_table_lookup_read_cell(
            "op_test stack read",
            constraint_builder,
            eid,
            move |meta| constant_from!(LocationType::Stack as u64),
            move |meta| sp.expr(meta) + constant_from!(1),
            move |meta| is_i32_cell.expr(meta),
            move |meta| value_cell.u64_cell.expr(meta),
            move |meta| constant_from!(1),
        );

        let memory_table_lookup_stack_write = allocator.alloc_memory_table_lookup_write_cell(
            "op_test stack write",
            constraint_builder,
            eid,
            move |meta| constant_from!(LocationType::Stack as u64),
            move |meta| sp.expr(meta) + constant_from!(1),
            move |meta| constant_from!(1),
            move |meta| res_cell.expr(meta),
            move |meta| constant_from!(1),
        );

        Box::new(TestConfig {
            is_i32_cell,
            res_cell,
            value_inv_cell,
            value_cell,
            memory_table_lookup_stack_read,
            memory_table_lookup_stack_write,
        })
    }
}

impl<F: FieldExt> EventTableOpcodeConfig<F> for TestConfig<F> {
    fn opcode(&self, meta: &mut VirtualCells<'_, F>) -> Expression<F> {
        constant!(bn_to_field(
            &(BigUint::from(OpcodeClass::Test as u64) << OPCODE_CLASS_SHIFT)
        )) + constant_from!(TestOp::Eqz as u16)
            * constant!(bn_to_field(&(BigUint::from(1u64) << OPCODE_ARG0_SHIFT)))
            + self.is_i32_cell.expr(meta)
                * constant!(bn_to_field(&(BigUint::from(1u64) << OPCODE_ARG1_SHIFT)))
    }

    fn assign(
        &self,
        ctx: &mut Context<'_, F>,
        step: &StepStatus,
        entry: &EventTableEntryWithMemoryInfo,
    ) -> Result<(), Error> {
        match &entry.eentry.step_info {
            StepInfo::Test {
                vtype,
                value,
                result,
            } => {
                self.is_i32_cell.assign_u32(ctx, *vtype as u32)?;

                self.value_cell.assign(ctx, *value)?;
                self.value_inv_cell
                    .assign(ctx, F::from(*value).invert().unwrap_or(F::zero()))?;
                self.res_cell.assign_u32(ctx, *result as u32)?;

                self.memory_table_lookup_stack_read.assign(
                    ctx,
                    entry.memory_rw_entires[0].start_eid,
                    step.current.eid,
                    entry.memory_rw_entires[0].end_eid,
                    step.current.sp + 1,
                    LocationType::Stack,
                    *vtype == VarType::I32,
                    *value,
                )?;

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

    fn memory_writing_ops(&self, entry: &EventTableEntry) -> u32 {
        1
    }
}
