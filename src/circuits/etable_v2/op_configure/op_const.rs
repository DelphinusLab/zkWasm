use crate::{
    circuits::{
        cell::*,
        etable_v2::{
            allocator::*, ConstraintBuilder, EventTableCommonConfig, EventTableOpcodeConfig,
            EventTableOpcodeConfigBuilder,
        },
        utils::{bn_to_field, step_status::StepStatus, Context},
    },
    constant, constant_from,
};
use halo2_proofs::{
    arithmetic::FieldExt,
    plonk::{Error, Expression, VirtualCells},
};
use num_bigint::BigUint;
use specs::{
    etable::EventTableEntry,
    itable::{OpcodeClass, OPCODE_ARG0_SHIFT, OPCODE_CLASS_SHIFT},
    mtable::LocationType,
};

pub struct ConstConfig<F: FieldExt> {
    is_i32: AllocatedBitCell<F>,
    value: AllocatedU64Cell<F>,
    lookup_stack_write: AllocatedMemoryTableLookupWriteCell<F>,
}

pub struct ConstConfigBuilder {}

impl<F: FieldExt> EventTableOpcodeConfigBuilder<F> for ConstConfigBuilder {
    fn configure(
        common_config: &EventTableCommonConfig<F>,
        allocator: &mut EventTableCellAllocator<F>,
        constraint_builder: &mut ConstraintBuilder<F>,
    ) -> Box<dyn EventTableOpcodeConfig<F>> {
        let is_i32 = allocator.alloc_bit_cell();
        let value = allocator.alloc_u64_cell();

        let sp_cell = common_config.sp_cell;
        let eid_cell = common_config.eid_cell;

        let lookup_stack_write = allocator.alloc_memory_table_lookup_write_cell(
            "op_const stack write",
            constraint_builder,
            eid_cell,
            move |_| constant_from!(LocationType::Stack as u64),
            move |meta| sp_cell.expr(meta),
            move |meta| is_i32.expr(meta),
            move |meta| value.u64_cell.expr(meta),
            move |_| constant_from!(1),
        );

        Box::new(ConstConfig {
            is_i32,
            value,
            lookup_stack_write,
        })
    }
}

impl<F: FieldExt> EventTableOpcodeConfig<F> for ConstConfig<F> {
    fn opcode(&self, meta: &mut VirtualCells<'_, F>) -> Expression<F> {
        constant!(bn_to_field(
            &(BigUint::from(OpcodeClass::Const as u64) << OPCODE_CLASS_SHIFT)
        )) + self.is_i32.expr(meta)
            * constant!(bn_to_field(&(BigUint::from(1u64) << OPCODE_ARG0_SHIFT)))
            + self.value.u64_cell.expr(meta)
    }

    fn assign(
        &self,
        ctx: &mut Context<'_, F>,
        step_info: &StepStatus,
        entry: &EventTableEntry,
    ) -> Result<(), Error> {
        match &entry.step_info {
            specs::step::StepInfo::I32Const { value } => {
                self.value.assign(ctx, *value as u32 as u64)?;
                self.is_i32.assign(ctx, F::one())?;
                /* TODO
                self.lookup_stack_write.assign(
                    ctx,
                    &MemoryTableLookupEncode::encode_stack_write(
                        BigUint::from(step_info.current.eid),
                        BigUint::from(1 as u64),
                        BigUint::from(step_info.current.sp),
                        BigUint::from(VarType::I32 as u16),
                        BigUint::from(*value as u32 as u64),
                    ),
                )?;
                */
                Ok(())
            }
            specs::step::StepInfo::I64Const { value } => {
                self.value.assign(ctx, *value as u64)?;
                self.is_i32.assign(ctx, F::zero())?;
                /* TODO
                self.lookup_stack_write.assign(
                    ctx,
                    &MemoryTableLookupEncode::encode_stack_write(
                        BigUint::from(step_info.current.eid),
                        BigUint::from(1 as u64),
                        BigUint::from(step_info.current.sp),
                        BigUint::from(VarType::I64 as u16),
                        BigUint::from(*value as u64),
                    ),
                )?;
                */
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
}
