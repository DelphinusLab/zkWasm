use super::*;
use crate::{
    circuits::{
        mtable_compact::encode::MemoryTableLookupEncode,
        rtable::{bits_of_offset_len, offset_len_bits_encode_expr, pow_table_encode},
        utils::{bn_to_field, Context},
    },
    constant,
};

use halo2_proofs::{
    arithmetic::FieldExt,
    plonk::{Error, Expression, VirtualCells},
};
use specs::{
    etable::EventTableEntry,
    itable::{OpcodeClass, OPCODE_ARG0_SHIFT, OPCODE_CLASS_SHIFT},
};
use specs::{mtable::VarType, step::StepInfo};

pub struct LoadConfig {
    opcode_load_offset: CommonRangeCell,

    load_start_block_index: CommonRangeCell,
    load_start_block_inner_offset: CommonRangeCell,
    load_start_block_inner_offset_helper: CommonRangeCell,

    load_end_block_index: CommonRangeCell,
    load_end_block_inner_offset: CommonRangeCell,
    load_end_block_inner_offset_helper: CommonRangeCell,

    load_value1: U64OnU8Cell,
    load_value2: U64OnU8Cell,

    mask_bits: [BitCell; 16],
    offset_modulus: U64Cell,
    res: U64Cell,
    load_base: U64Cell,

    is_one_byte: BitCell,
    is_two_bytes: BitCell,
    is_four_bytes: BitCell,
    is_eight_bytes: BitCell,
    is_sign: BitCell,

    lookup_stack_read: MTableLookupCell,
    lookup_heap_read1: MTableLookupCell,
    lookup_heap_read2: MTableLookupCell,
    lookup_stack_write: MTableLookupCell,

    lookup_offset_len_bits: OffsetLenBitsTableLookupCell,
    lookup_pow: PowTableLookupCell,
}

pub struct LoadConfigBuilder {}

impl<F: FieldExt> EventTableOpcodeConfigBuilder<F> for LoadConfigBuilder {
    fn configure(
        common: &mut EventTableCellAllocator<F>,
        constraint_builder: &mut ConstraintBuilder<F>,
    ) -> Box<dyn EventTableOpcodeConfig<F>> {
        let opcode_load_offset = common.alloc_common_range_value();

        let load_start_block_index = common.alloc_common_range_value();
        let load_start_block_inner_offset = common.alloc_common_range_value();
        let load_start_block_inner_offset_helper = common.alloc_common_range_value();

        let load_end_block_index = common.alloc_common_range_value();
        let load_end_block_inner_offset = common.alloc_common_range_value();
        let load_end_block_inner_offset_helper = common.alloc_common_range_value();

        let load_value1 = common.alloc_u64_on_u8();
        let load_value2 = common.alloc_u64_on_u8();
        let offset_modulus = common.alloc_u64();
        let res = common.alloc_u64();
        let load_base = common.alloc_u64();

        let mask_bits = [0; 16].map(|_| common.alloc_bit_value());
        let is_one_byte = common.alloc_bit_value();
        let is_two_bytes = common.alloc_bit_value();
        let is_four_bytes = common.alloc_bit_value();
        let is_eight_bytes = common.alloc_bit_value();
        let is_sign = common.alloc_bit_value();

        let lookup_stack_read = common.alloc_mtable_lookup();
        let lookup_heap_read1 = common.alloc_mtable_lookup();
        let lookup_heap_read2 = common.alloc_mtable_lookup();
        let lookup_stack_write = common.alloc_mtable_lookup();

        let lookup_offset_len_bits = common.alloc_offset_len_bits_table_lookup();
        let lookup_pow = common.alloc_pow_table_lookup();

        constraint_builder.push(
            "op_load start end offset range",
            Box::new(move |meta| {
                vec![
                    load_start_block_inner_offset.expr(meta)
                        + load_start_block_inner_offset_helper.expr(meta)
                        - constant_from!(8),
                    load_end_block_inner_offset.expr(meta)
                        + load_end_block_inner_offset_helper.expr(meta)
                        - constant_from!(8),
                ]
            }),
        );

        constraint_builder.push(
            "op_load start end equation",
            Box::new(move |meta| {
                let len = constant_from!(1)
                    + is_two_bytes.expr(meta) * constant_from!(1)
                    + is_four_bytes.expr(meta) * constant_from!(3)
                    + is_eight_bytes.expr(meta) * constant_from!(7);
                vec![
                    load_start_block_index.expr(meta) * constant_from!(8)
                        + load_start_block_inner_offset.expr(meta)
                        + len
                        - constant_from!(1)
                        - load_end_block_index.expr(meta) * constant_from!(8)
                        - load_end_block_inner_offset.expr(meta),
                ]
            }),
        );

        constraint_builder.push(
            "op_load start load_base",
            Box::new(move |meta| {
                vec![
                    load_base.expr(meta) + opcode_load_offset.expr(meta)
                        - load_start_block_index.expr(meta) * constant_from!(8)
                        - load_start_block_inner_offset.expr(meta),
                ]
            }),
        );

        constraint_builder.push(
            "op_load length",
            Box::new(move |meta| {
                vec![
                    is_one_byte.expr(meta)
                        + is_two_bytes.expr(meta)
                        + is_four_bytes.expr(meta)
                        + is_eight_bytes.expr(meta)
                        - constant_from!(1),
                ]
            }),
        );

        constraint_builder.push(
            "op_load mask_bits offset len",
            Box::new(move |meta| {
                let len = constant_from!(1)
                    + is_two_bytes.expr(meta) * constant_from!(1)
                    + is_four_bytes.expr(meta) * constant_from!(3)
                    + is_eight_bytes.expr(meta) * constant_from!(7);

                let (_, bits_encode) = mask_bits
                    .map(|c| c.expr(meta))
                    .into_iter()
                    .enumerate()
                    .reduce(|(_, acc), (i, e)| (i, acc + e * constant_from!(1u64 << i)))
                    .unwrap();
                vec![
                    lookup_offset_len_bits.expr(meta)
                        - offset_len_bits_encode_expr(
                            load_start_block_inner_offset.expr(meta),
                            len,
                            bits_encode,
                        ),
                ]
            }),
        );

        constraint_builder.push(
            "op_load pow table lookup",
            Box::new(move |meta| {
                vec![
                    lookup_pow.expr(meta)
                        - pow_table_encode(
                            offset_modulus.expr(meta),
                            load_start_block_inner_offset.expr(meta) * constant_from!(8),
                        ),
                ]
            }),
        );

        constraint_builder.push(
            "op_load values",
            Box::new(move |meta| {
                let mut acc = res.expr(meta) * offset_modulus.expr(meta);

                for i in 0..8 {
                    acc = acc
                        - load_value1.u8_expr(meta, i)
                            * constant!(bn_to_field(&(BigUint::from(1u64) << (i * 8))))
                            * mask_bits[i as usize].expr(meta);

                    acc = acc
                        - load_value2.u8_expr(meta, i)
                            * constant!(bn_to_field(&(BigUint::from(1u64) << (i * 8 + 64))))
                            * mask_bits[i as usize + 8].expr(meta);
                }

                vec![acc]
            }),
        );

        Box::new(LoadConfig {
            opcode_load_offset,
            load_start_block_index,
            load_start_block_inner_offset,
            load_start_block_inner_offset_helper,
            load_end_block_index,
            load_end_block_inner_offset,
            load_end_block_inner_offset_helper,
            load_value1,
            load_value2,
            mask_bits,
            offset_modulus,
            load_base,
            res,
            is_one_byte,
            is_two_bytes,
            is_four_bytes,
            is_eight_bytes,
            is_sign,
            lookup_stack_read,
            lookup_heap_read1,
            lookup_heap_read2,
            lookup_stack_write,
            lookup_offset_len_bits,
            lookup_pow,
        })
    }
}

impl<F: FieldExt> EventTableOpcodeConfig<F> for LoadConfig {
    fn opcode(&self, meta: &mut VirtualCells<'_, F>) -> Expression<F> {
        let vtype = self.is_eight_bytes.expr(meta) * constant_from!(6)
            + self.is_four_bytes.expr(meta) * constant_from!(4)
            + self.is_two_bytes.expr(meta) * constant_from!(2)
            + self.is_sign.expr(meta)
            + constant_from!(1);

        constant!(bn_to_field(
            &(BigUint::from(OpcodeClass::Load as u64) << OPCODE_CLASS_SHIFT)
        )) + vtype * constant!(bn_to_field(&(BigUint::from(1u64) << OPCODE_ARG0_SHIFT)))
            + self.opcode_load_offset.expr(meta)
    }

    fn assign(
        &self,
        ctx: &mut Context<'_, F>,
        step_info: &StepStatus,
        entry: &EventTableEntry,
    ) -> Result<(), Error> {
        match entry.step_info {
            StepInfo::Load {
                vtype,
                offset,
                raw_address,
                effective_address,
                value,
                block_value,
                mmid,
            } => {
                self.opcode_load_offset
                    .assign(ctx, offset.try_into().unwrap())?;

                let len = vtype.byte_size();
                let start_byte_index = effective_address as u64;
                let end_byte_index = start_byte_index + len - 1;

                self.load_start_block_index
                    .assign(ctx, (start_byte_index / 8).try_into().unwrap())?;
                self.load_start_block_inner_offset
                    .assign(ctx, (start_byte_index % 8).try_into().unwrap())?;
                self.load_start_block_inner_offset_helper
                    .assign(ctx, (8 - start_byte_index % 8).try_into().unwrap())?;

                self.load_end_block_index
                    .assign(ctx, (end_byte_index / 8).try_into().unwrap())?;
                self.load_end_block_inner_offset
                    .assign(ctx, (end_byte_index % 8).try_into().unwrap())?;
                self.load_end_block_inner_offset_helper
                    .assign(ctx, (8 - end_byte_index % 8).try_into().unwrap())?;

                self.load_value1.assign(ctx, block_value)?;
                // TODO replace 0 if cross load
                self.load_value2.assign(ctx, 0)?;

                let offset = start_byte_index % 8;
                let bits = bits_of_offset_len(offset, len);
                for i in 0..16 {
                    self.mask_bits[i].assign(ctx, (bits >> i) & 1 == 1)?;
                }
                self.offset_modulus.assign(ctx, 1 << (offset * 8))?;
                self.res.assign(ctx, value)?;
                self.load_base.assign(ctx, raw_address.into())?;

                self.is_one_byte.assign(ctx, len == 1)?;
                self.is_two_bytes.assign(ctx, len == 2)?;
                self.is_four_bytes.assign(ctx, len == 4)?;
                self.is_eight_bytes.assign(ctx, len == 8)?;
                self.is_sign.assign(ctx, vtype.is_sign())?;

                self.lookup_stack_read.assign(
                    ctx,
                    &MemoryTableLookupEncode::encode_stack_read(
                        BigUint::from(step_info.current.eid),
                        BigUint::from(1 as u64),
                        BigUint::from(step_info.current.sp + 1),
                        BigUint::from(VarType::I32 as u16),
                        BigUint::from(raw_address),
                    ),
                )?;

                self.lookup_heap_read1.assign(
                    ctx,
                    &MemoryTableLookupEncode::encode_memory_load(
                        BigUint::from(step_info.current.eid),
                        BigUint::from(2 as u64),
                        BigUint::from(mmid),
                        BigUint::from(start_byte_index / 8),
                        BigUint::from(VarType::U64 as u16),
                        BigUint::from(block_value),
                    ),
                )?;

                //TODO: assign for cross load block value

                self.lookup_stack_write.assign(
                    ctx,
                    &MemoryTableLookupEncode::encode_stack_write(
                        BigUint::from(step_info.current.eid),
                        //TODO: may be 4
                        BigUint::from(3 as u64),
                        BigUint::from(step_info.current.sp + 1),
                        BigUint::from(vtype as u16),
                        BigUint::from(value),
                    ),
                )?;

                self.lookup_offset_len_bits.assign(ctx, offset, len)?;
                self.lookup_pow.assign(ctx, offset * 8)?;

                Ok(())
            }

            _ => unreachable!(),
        }
    }

    fn opcode_class(&self) -> OpcodeClass {
        OpcodeClass::Load
    }

    fn mops(&self, meta: &mut VirtualCells<'_, F>) -> Option<Expression<F>> {
        Some(
            constant_from!(3) + self.load_end_block_index.expr(meta)
                - self.load_start_block_index.expr(meta),
        )
    }

    fn mtable_lookup(
        &self,
        meta: &mut VirtualCells<'_, F>,
        item: MLookupItem,
        common_config: &EventTableCommonConfig<F>,
    ) -> Option<Expression<F>> {
        let vtype = self.is_eight_bytes.expr(meta) * constant_from!(6)
            + self.is_four_bytes.expr(meta) * constant_from!(4)
            + self.is_two_bytes.expr(meta) * constant_from!(2)
            + self.is_sign.expr(meta)
            + constant_from!(1);

        let cross_load =
            self.load_end_block_index.expr(meta) - self.load_start_block_index.expr(meta);

        match item {
            MLookupItem::First => Some(MemoryTableLookupEncode::encode_stack_read(
                common_config.eid(meta),
                constant_from!(1),
                common_config.sp(meta) + constant_from!(1),
                constant_from!(VarType::I32),
                self.load_base.expr(meta),
            )),
            MLookupItem::Second => Some(MemoryTableLookupEncode::encode_memory_load(
                common_config.eid(meta),
                constant_from!(2),
                common_config.mmid(meta),
                self.load_start_block_index.expr(meta),
                constant_from!(VarType::U64),
                self.load_value1.expr(meta),
            )),
            MLookupItem::Third => Some(
                MemoryTableLookupEncode::encode_memory_load(
                    common_config.eid(meta),
                    constant_from!(3),
                    common_config.mmid(meta),
                    self.load_end_block_index.expr(meta),
                    constant_from!(VarType::U64),
                    self.load_value2.expr(meta),
                ) * cross_load.clone(),
            ),
            MLookupItem::Fourth => Some(MemoryTableLookupEncode::encode_stack_write(
                common_config.eid(meta),
                constant_from!(3) + cross_load,
                common_config.sp(meta) + constant_from!(1),
                vtype,
                self.res.expr(meta),
            )),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::test::test_circuit_builder::test_circuit_noexternal;

    #[test]
    fn test_load() {
        let textual_repr = r#"
                (module
                    (memory $0 1)
                    (data (i32.const 0) "\ff\00\00\00\fe\00\00\00")
                    (func (export "test")
                      (i32.const 0)
                      (i32.load offset=0)
                      (drop)
                      (i32.const 4)
                      (i32.load offset=0)
                      (drop)
                    )
                   )
                "#;

        test_circuit_noexternal(textual_repr).unwrap();
    }
}
