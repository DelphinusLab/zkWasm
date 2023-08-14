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
use specs::encode::opcode::encode_conversion;
use specs::etable::EventTableEntry;
use specs::itable::ConversionOp;
use specs::mtable::LocationType;
use specs::mtable::VarType;
use specs::step::StepInfo;

pub struct ConversionConfig<F: FieldExt> {
    value: AllocatedU64CellWithFlagBit<F, 1>,
    value_is_i32: AllocatedBitCell<F>,
    res: AllocatedU64Cell<F>,
    res_is_i32: AllocatedBitCell<F>,

    is_i32_wrap_i64: AllocatedBitCell<F>,
    is_i64_extend_i32_u: AllocatedBitCell<F>,
    is_i64_extend_i32_s: AllocatedBitCell<F>,

    memory_table_lookup_stack_read: AllocatedMemoryTableLookupReadCell<F>,
    memory_table_lookup_stack_write: AllocatedMemoryTableLookupWriteCell<F>,
}

pub struct ConversionConfigBuilder {}

impl<F: FieldExt> EventTableOpcodeConfigBuilder<F> for ConversionConfigBuilder {
    fn configure(
        common_config: &EventTableCommonConfig<F>,
        allocator: &mut EventTableCellAllocator<F>,
        constraint_builder: &mut ConstraintBuilder<F>,
    ) -> Box<dyn EventTableOpcodeConfig<F>> {
        let value = allocator.alloc_u64_with_flag_bit_cell(constraint_builder);
        let value_is_i32 = allocator.alloc_bit_cell();
        let res = allocator.alloc_u64_cell();
        let res_is_i32 = allocator.alloc_bit_cell();

        let is_i32_wrap_i64 = allocator.alloc_bit_cell();
        let is_i64_extend_i32_u = allocator.alloc_bit_cell();
        let is_i64_extend_i32_s = allocator.alloc_bit_cell();

        constraint_builder.push(
            "op_conversion pick one",
            Box::new(move |meta| {
                vec![
                    is_i32_wrap_i64.expr(meta)
                        + is_i64_extend_i32_u.expr(meta)
                        + is_i64_extend_i32_s.expr(meta)
                        - constant_from!(1),
                ]
            }),
        );

        constraint_builder.push(
            "op_conversion type matches op",
            Box::new(move |meta| {
                vec![
                    is_i32_wrap_i64.expr(meta) * value_is_i32.expr(meta),
                    is_i32_wrap_i64.expr(meta) * (res_is_i32.expr(meta) - constant_from!(1)),
                    (is_i64_extend_i32_s.expr(meta) + is_i64_extend_i32_u.expr(meta))
                        * (value_is_i32.expr(meta) - constant_from!(1)),
                    (is_i64_extend_i32_s.expr(meta) + is_i64_extend_i32_u.expr(meta))
                        * res_is_i32.expr(meta),
                ]
            }),
        );

        constraint_builder.push(
            "op_conversion i32_wrap_i64",
            Box::new(move |meta| {
                vec![
                    is_i32_wrap_i64.expr(meta)
                        * (value.u16_cells_le[1].expr(meta) * constant_from!(1 << 16)
                            + value.u16_cells_le[0].expr(meta)
                            - res.u64_cell.expr(meta)),
                ]
            }),
        );

        constraint_builder.push(
            "i64_extend_i32_u",
            Box::new(move |meta| {
                vec![
                    is_i64_extend_i32_u.expr(meta)
                        * (res.u64_cell.expr(meta) - value.u64_cell.expr(meta)),
                ]
            }),
        );

        constraint_builder.push(
            "i64_extend_i32_s",
            Box::new(move |meta| {
                let pad = value.flag_bit_cell.expr(meta) * constant_from!((u32::MAX as u64) << 32);

                vec![
                    is_i64_extend_i32_s.expr(meta)
                        * (pad + value.u64_cell.expr(meta) - res.u64_cell.expr(meta)),
                ]
            }),
        );

        let eid = common_config.eid_cell;
        let sp = common_config.sp_cell;

        let memory_table_lookup_stack_read = allocator.alloc_memory_table_lookup_read_cell(
            "op_conversion stack read",
            constraint_builder,
            eid,
            move |____| constant_from!(LocationType::Stack as u64),
            move |meta| sp.expr(meta) + constant_from!(1),
            move |meta| value_is_i32.expr(meta),
            move |meta| value.u64_cell.expr(meta),
            move |____| constant_from!(1),
        );

        let memory_table_lookup_stack_write = allocator.alloc_memory_table_lookup_write_cell(
            "op_conversion stack write",
            constraint_builder,
            eid,
            move |____| constant_from!(LocationType::Stack as u64),
            move |meta| sp.expr(meta) + constant_from!(1),
            move |meta| res_is_i32.expr(meta),
            move |meta| res.u64_cell.expr(meta),
            move |____| constant_from!(1),
        );

        Box::new(ConversionConfig {
            value,
            value_is_i32,
            res,
            res_is_i32,
            is_i32_wrap_i64,
            is_i64_extend_i32_u,
            is_i64_extend_i32_s,
            memory_table_lookup_stack_read,
            memory_table_lookup_stack_write,
        })
    }
}

impl<F: FieldExt> EventTableOpcodeConfig<F> for ConversionConfig<F> {
    fn opcode(&self, meta: &mut VirtualCells<'_, F>) -> Expression<F> {
        self.is_i32_wrap_i64.expr(meta)
            * encode_conversion::<Expression<F>>(ConversionOp::I32WrapI64)
            + self.is_i64_extend_i32_s.expr(meta)
                * encode_conversion::<Expression<F>>(ConversionOp::I64ExtendI32s)
            + self.is_i64_extend_i32_u.expr(meta)
                * encode_conversion::<Expression<F>>(ConversionOp::I64ExtendI32u)
    }

    fn assign(
        &self,
        ctx: &mut Context<'_, F>,
        step: &StepStatus,
        entry: &EventTableEntryWithMemoryInfo,
    ) -> Result<(), Error> {
        let (value, value_type, result, result_type) = match &entry.eentry.step_info {
            StepInfo::I32WrapI64 { value, result } => {
                self.is_i32_wrap_i64.assign(ctx, F::one())?;

                (
                    *value as u64,
                    VarType::I64,
                    *result as u32 as u64,
                    VarType::I32,
                )
            }
            StepInfo::I64ExtendI32 {
                value,
                result,
                sign,
            } => {
                if *sign {
                    self.is_i64_extend_i32_s.assign(ctx, F::one())?;
                } else {
                    self.is_i64_extend_i32_u.assign(ctx, F::one())?;
                }

                (
                    *value as u32 as u64,
                    VarType::I32,
                    *result as u64,
                    VarType::I64,
                )
            }
            _ => unreachable!(),
        };

        self.value.assign(ctx, value)?;
        self.res.assign(ctx, result)?;
        self.value_is_i32.assign(ctx, F::from(value_type as u64))?;
        self.res_is_i32.assign(ctx, F::from(result_type as u64))?;

        self.memory_table_lookup_stack_read.assign(
            ctx,
            entry.memory_rw_entires[0].start_eid,
            step.current.eid,
            entry.memory_rw_entires[0].end_eid,
            step.current.sp + 1,
            LocationType::Stack,
            value_type == VarType::I32,
            value,
        )?;

        self.memory_table_lookup_stack_write.assign(
            ctx,
            step.current.eid,
            entry.memory_rw_entires[1].end_eid,
            step.current.sp + 1,
            LocationType::Stack,
            result_type == VarType::I32,
            result,
        )?;

        Ok(())
    }

    fn mops(&self, _meta: &mut VirtualCells<'_, F>) -> Option<Expression<F>> {
        Some(constant_from!(1))
    }

    fn memory_writing_ops(&self, _: &EventTableEntry) -> u32 {
        1
    }
}
