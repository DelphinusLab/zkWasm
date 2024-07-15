use crate::circuits::bit_table::BitTableOp;
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
use specs::encode::opcode::encode_bin_bit;
use specs::encode::opcode::UniArgEncode;
use specs::etable::EventTableEntry;
use specs::itable::BitOp;
use specs::itable::Opcode;
use specs::itable::UniArg;
use specs::mtable::LocationType;
use specs::mtable::VarType;
use specs::step::StepInfo;

pub struct BinBitConfig<F: FieldExt> {
    op_class: AllocatedCommonRangeCell<F>,
    bit_table_lookup: AllocatedBitTableLookupCells<F>,

    lhs: EventTableCommonArgsConfig<F>,
    rhs: EventTableCommonArgsConfig<F>,
    memory_table_lookup_stack_write: AllocatedMemoryTableLookupWriteCell<F>,
}

pub struct BinBitConfigBuilder {}

impl<F: FieldExt> EventTableOpcodeConfigBuilder<F> for BinBitConfigBuilder {
    fn configure(
        common_config: &EventTableCommonConfig<F>,
        allocator: &mut EventTableCellAllocator<F>,
        constraint_builder: &mut ConstraintBuilder<F>,
    ) -> Box<dyn EventTableOpcodeConfig<F>> {
        let op_class = allocator.alloc_common_range_cell();

        let eid = common_config.eid_cell;
        let sp = common_config.sp_cell;
        let bit_table_lookup = common_config.bit_table_lookup_cells;

        let rhs = common_config.uniarg_configs[0].clone();
        let lhs = common_config.uniarg_configs[1].clone();

        let memory_table_lookup_stack_write = allocator
            .alloc_memory_table_lookup_write_cell_with_value(
                "op_bin stack write",
                constraint_builder,
                eid,
                move |____| constant_from!(LocationType::Stack as u64),
                move |meta| sp.expr(meta) + rhs.is_pop_cell.expr(meta) + lhs.is_pop_cell.expr(meta),
                move |meta| rhs.is_i32_cell.expr(meta),
                move |____| constant_from!(1),
            );
        let res = memory_table_lookup_stack_write.value_cell;

        {
            let tmp = common_config.uniarg_configs[2].is_enabled_cell.clone();
            constraint_builder.push(
                "op_bin_bit: args",
                Box::new(move |meta| {
                    vec![
                        rhs.is_i32_cell.expr(meta) - lhs.is_i32_cell.expr(meta),
                        rhs.is_enabled_cell.expr(meta) + lhs.is_enabled_cell.expr(meta)
                            - constant_from!(2),
                        // disable unused uniarg
                        tmp.expr(meta),
                    ]
                }),
            );
        }

        constraint_builder.push(
            "op_bin_bit: lookup",
            Box::new(move |meta| {
                vec![
                    bit_table_lookup.op.expr(meta) - op_class.expr(meta),
                    bit_table_lookup.left.expr(meta) - lhs.value_cell.expr(meta),
                    bit_table_lookup.right.expr(meta) - rhs.value_cell.expr(meta),
                    bit_table_lookup.result.expr(meta) - res.expr(meta),
                ]
            }),
        );

        Box::new(BinBitConfig {
            op_class,
            bit_table_lookup,
            lhs,
            rhs,
            memory_table_lookup_stack_write,
        })
    }
}

impl<F: FieldExt> EventTableOpcodeConfig<F> for BinBitConfig<F> {
    fn opcode(&self, meta: &mut VirtualCells<'_, F>) -> Expression<F> {
        encode_bin_bit(
            self.op_class.expr(meta),
            self.rhs.is_i32_cell.expr(meta),
            UniArgEncode::Reserve,
        )
    }

    fn assign(
        &self,
        ctx: &mut Context<'_, F>,
        step: &mut StepStatus<F>,
        entry: &EventTableEntryWithMemoryInfo,
    ) -> Result<(), Error> {
        let (class, vtype, left, right, value) = match entry.eentry.step_info {
            StepInfo::I32BinBitOp {
                class,
                left,
                right,
                value,
            } => {
                let vtype = VarType::I32;
                let left = left as u32 as u64;
                let right = right as u32 as u64;
                let value = value as u32 as u64;
                (class, vtype, left, right, value)
            }
            StepInfo::I64BinBitOp {
                class,
                left,
                right,
                value,
            } => {
                let vtype = VarType::I64;
                let left = left as u64;
                let right = right as u64;
                let value = value as u64;
                (class, vtype, left, right, value)
            }
            _ => unreachable!(),
        };

        self.bit_table_lookup
            .assign(ctx, BitTableOp::BinaryBit(class), left, right, value)?;

        match class {
            specs::itable::BitOp::And => {
                self.op_class.assign_u32(ctx, BitOp::And as u32)?;
            }
            specs::itable::BitOp::Or => {
                self.op_class.assign_u32(ctx, BitOp::Or as u32)?;
            }
            specs::itable::BitOp::Xor => {
                self.op_class.assign_u32(ctx, BitOp::Xor as u32)?;
            }
        };

        if let Opcode::BinBit {
            class,
            vtype,
            uniargs,
        } = entry.eentry.get_instruction(&step.current.itable).opcode
        {
            let mut memory_entries = entry.memory_rw_entires.iter();

            self.rhs.assign(ctx, uniargs[0], &mut memory_entries)?;
            self.lhs.assign(ctx, uniargs[1], &mut memory_entries)?;
            self.memory_table_lookup_stack_write
                .assign_with_memory_entry(ctx, &mut memory_entries)?;
        } else {
            unreachable!();
        }

        Ok(())
    }

    fn mops(&self, _meta: &mut VirtualCells<'_, F>) -> Option<Expression<F>> {
        Some(constant_from!(1))
    }

    fn memory_writing_ops(&self, _: &EventTableEntry) -> u32 {
        1
    }

    fn sp_diff(&self, _meta: &mut VirtualCells<'_, F>) -> Option<Expression<F>> {
        Some(constant!(F::one()))
    }
}
