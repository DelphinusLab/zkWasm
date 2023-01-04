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
    configure_table::WASM_PAGE_SIZE,
    etable::EventTableEntry,
    itable::{OpcodeClass, OPCODE_ARG0_SHIFT, OPCODE_ARG1_SHIFT, OPCODE_CLASS_SHIFT},
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
    offset_modulus: U64OnU8Cell,
    res: U64Cell,
    value_in_heap: U64Cell,
    load_base: U64Cell,

    vtype: CommonRangeCell,
    is_one_byte: BitCell,
    is_two_bytes: BitCell,
    is_four_bytes: BitCell,
    is_eight_bytes: BitCell,
    is_sign: BitCell,
    is_i64: BitCell,

    highest_u4: [BitCell; 4],

    lookup_stack_read: MTableLookupCell,
    lookup_heap_read1: MTableLookupCell,
    lookup_heap_read2: MTableLookupCell,
    lookup_stack_write: MTableLookupCell,

    lookup_offset_len_bits: OffsetLenBitsTableLookupCell,
    lookup_pow: PowTableLookupCell,

    address_within_allocated_pages_helper: CommonRangeCell,
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
        let offset_modulus = common.alloc_u64_on_u8();
        let res = common.alloc_u64();
        let value_in_heap = common.alloc_u64();
        let load_base = common.alloc_u64();

        let mask_bits = [0; 16].map(|_| common.alloc_bit_value());
        let is_one_byte = common.alloc_bit_value();
        let is_two_bytes = common.alloc_bit_value();
        let is_four_bytes = common.alloc_bit_value();
        let is_eight_bytes = common.alloc_bit_value();
        let is_sign = common.alloc_bit_value();
        let is_i64 = common.alloc_bit_value();
        let vtype = common.alloc_common_range_value();

        let highest_u4 = [0; 4].map(|_| common.alloc_bit_value());

        let lookup_stack_read = common.alloc_mtable_lookup();
        let lookup_heap_read1 = common.alloc_mtable_lookup();
        let lookup_heap_read2 = common.alloc_mtable_lookup();
        let lookup_stack_write = common.alloc_mtable_lookup();

        let lookup_offset_len_bits = common.alloc_offset_len_bits_table_lookup();
        let lookup_pow = common.alloc_pow_table_lookup();

        let current_memory_page_size = common.allocated_memory_pages_cell();
        let address_within_allocated_pages_helper = common.alloc_common_range_value();

        constraint_builder.push(
            "op_load start end offset <= 7",
            Box::new(move |meta| {
                vec![
                    load_start_block_inner_offset.expr(meta)
                        + load_start_block_inner_offset_helper.expr(meta)
                        - constant_from!(7),
                    load_end_block_inner_offset.expr(meta)
                        + load_end_block_inner_offset_helper.expr(meta)
                        - constant_from!(7),
                ]
            }),
        );

        constraint_builder.push(
            "op_load start end equation, start_index * 8 + start_offset + len = stop_index * 8 + stop_offset + 1",
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
            "op_load value_in_heap",
            Box::new(move |meta| {
                let mut acc = value_in_heap.expr(meta) * offset_modulus.expr(meta);

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

        constraint_builder.push(
            "op_load value: value = padding + value_in_heap",
            Box::new(move |meta| {
                let mut acc = is_one_byte.expr(meta) * value_in_heap.u4_expr(meta, 1)
                    + is_two_bytes.expr(meta) * value_in_heap.u4_expr(meta, 3)
                    + is_four_bytes.expr(meta) * value_in_heap.u4_expr(meta, 7)
                    + is_eight_bytes.expr(meta) * value_in_heap.u4_expr(meta, 15);
                for i in 0..4 {
                    acc = acc - highest_u4[i].expr(meta) * constant_from!(1u64 << 3 - i as u64)
                }
                let padding = is_one_byte.expr(meta) * constant_from!(0xffffff00)
                    + is_two_bytes.expr(meta) * constant_from!(0xffff0000)
                    + (constant_from!(1) - is_eight_bytes.expr(meta))
                        * is_i64.expr(meta)
                        * constant_from!(0xffffffff00000000);
                vec![
                    res.expr(meta)
                        - value_in_heap.expr(meta)
                        - highest_u4[0].expr(meta) * is_sign.expr(meta) * padding,
                    acc,
                ]
            }),
        );

        constraint_builder.push(
            "op_load: is_i64 = 1 when vtype = 2",
            Box::new(move |meta| vec![is_i64.expr(meta) + constant_from!(1) - vtype.expr(meta)]),
        );

        constraint_builder.push(
            "op_load allocated address",
            Box::new(move |meta| {
                let len = constant_from!(1)
                    + is_two_bytes.expr(meta) * constant_from!(1)
                    + is_four_bytes.expr(meta) * constant_from!(3)
                    + is_eight_bytes.expr(meta) * constant_from!(7);

                vec![
                    (load_base.expr(meta)
                        + opcode_load_offset.expr(meta)
                        + len
                        + address_within_allocated_pages_helper.expr(meta)
                        - current_memory_page_size.expr(meta) * constant_from!(WASM_PAGE_SIZE)),
                ]
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
            value_in_heap,
            is_one_byte,
            is_two_bytes,
            is_four_bytes,
            is_eight_bytes,
            is_sign,
            is_i64,
            highest_u4,
            vtype,
            lookup_stack_read,
            lookup_heap_read1,
            lookup_heap_read2,
            lookup_stack_write,
            lookup_offset_len_bits,
            lookup_pow,
            address_within_allocated_pages_helper,
        })
    }
}

impl<F: FieldExt> EventTableOpcodeConfig<F> for LoadConfig {
    fn opcode(&self, meta: &mut VirtualCells<'_, F>) -> Expression<F> {
        let load_size = self.is_eight_bytes.expr(meta) * constant_from!(6)
            + self.is_four_bytes.expr(meta) * constant_from!(4)
            + self.is_two_bytes.expr(meta) * constant_from!(2)
            + self.is_sign.expr(meta)
            + constant_from!(1);

        constant!(bn_to_field(
            &(BigUint::from(OpcodeClass::Load as u64) << OPCODE_CLASS_SHIFT)
        )) + self.vtype.expr(meta)
            * constant!(bn_to_field(&(BigUint::from(1u64) << OPCODE_ARG0_SHIFT)))
            + load_size * constant!(bn_to_field(&(BigUint::from(1u64) << OPCODE_ARG1_SHIFT)))
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
                load_size,
                offset,
                raw_address,
                effective_address,
                value,
                block_value1,
                block_value2,
                mmid,
            } => {
                self.opcode_load_offset
                    .assign(ctx, F::from(offset as u64))?;

                let len = load_size.byte_size();
                let start_byte_index = effective_address as u64;
                let end_byte_index = start_byte_index + len - 1;

                self.load_start_block_index
                    .assign(ctx, (start_byte_index / 8).try_into().unwrap())?;
                self.load_start_block_inner_offset
                    .assign(ctx, (start_byte_index % 8).try_into().unwrap())?;
                self.load_start_block_inner_offset_helper
                    .assign(ctx, (7 - start_byte_index % 8).try_into().unwrap())?;

                self.load_end_block_index
                    .assign(ctx, (end_byte_index / 8).try_into().unwrap())?;
                self.load_end_block_inner_offset
                    .assign(ctx, (end_byte_index % 8).try_into().unwrap())?;
                self.load_end_block_inner_offset_helper
                    .assign(ctx, (7 - end_byte_index % 8).try_into().unwrap())?;

                self.load_value1.assign(ctx, block_value1)?;
                self.load_value2.assign(ctx, block_value2)?;

                let offset = start_byte_index % 8;
                let bits = bits_of_offset_len(offset, len);
                for i in 0..16 {
                    self.mask_bits[i].assign(ctx, (bits >> i) & 1 == 1)?;
                }
                self.offset_modulus.assign(ctx, 1 << (offset * 8))?;
                self.load_base.assign(ctx, raw_address.into())?;

                let mut mask: u64 = 0;
                for _ in 0..len {
                    mask = (mask << 8) + 0xff;
                }
                let highest_bit = value >> vtype as u64 * 32 - 1;
                let value_in_heap = if load_size.is_sign() && highest_bit == 1 {
                    value & mask
                } else {
                    value
                };
                self.value_in_heap.assign(ctx, value_in_heap)?;
                self.res.assign(ctx, value)?;

                for i in 0..4 {
                    self.highest_u4[i]
                        .assign(ctx, (value_in_heap >> 8 * len - i as u64 - 1) & 1 == 1)?;
                }

                self.is_one_byte.assign(ctx, len == 1)?;
                self.is_two_bytes.assign(ctx, len == 2)?;
                self.is_four_bytes.assign(ctx, len == 4)?;
                self.is_eight_bytes.assign(ctx, len == 8)?;
                self.is_sign.assign(ctx, load_size.is_sign())?;
                self.is_i64.assign(ctx, vtype == VarType::I64)?;
                self.vtype.assign(ctx, F::from(vtype as u64))?;

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
                        BigUint::from(VarType::I64 as u16),
                        BigUint::from(block_value1),
                    ),
                )?;

                if offset + len > 8 {
                    self.lookup_heap_read2.assign(
                        ctx,
                        &MemoryTableLookupEncode::encode_memory_load(
                            BigUint::from(step_info.current.eid),
                            BigUint::from(3 as u64),
                            BigUint::from(mmid),
                            BigUint::from(end_byte_index / 8),
                            BigUint::from(VarType::I64 as u16),
                            BigUint::from(block_value2),
                        ),
                    )?;
                }

                self.lookup_stack_write.assign(
                    ctx,
                    &MemoryTableLookupEncode::encode_stack_write(
                        BigUint::from(step_info.current.eid),
                        BigUint::from(3 + (offset + len - 1) / 8 as u64),
                        BigUint::from(step_info.current.sp + 1),
                        BigUint::from(vtype as u16),
                        BigUint::from(value),
                    ),
                )?;

                self.address_within_allocated_pages_helper.assign(
                    ctx,
                    F::from(
                        step_info.current.allocated_memory_pages as u64 * WASM_PAGE_SIZE
                            - (effective_address as u64 + len),
                    ),
                )?;

                self.lookup_offset_len_bits.assign(ctx, offset, len)?;
                self.lookup_pow.assign(ctx, offset * 8)?;

                Ok(())
            }

            _ => unreachable!(),
        }
    }

    fn mops(&self, meta: &mut VirtualCells<'_, F>) -> Option<Expression<F>> {
        Some(
            constant_from!(3) + self.load_end_block_index.expr(meta)
                - self.load_start_block_index.expr(meta),
        )
    }

    fn assigned_extra_mops(
        &self,
        _ctx: &mut Context<'_, F>,
        _step: &StepStatus,
        entry: &EventTableEntry,
    ) -> u64 {
        match &entry.step_info {
            StepInfo::Load {
                load_size,
                effective_address,
                ..
            } => {
                if (*effective_address + load_size.byte_size() as u32 - 1) / 8
                    != *effective_address / 8
                {
                    1
                } else {
                    0
                }
            }
            _ => unreachable!(),
        }
    }

    fn mtable_lookup(
        &self,
        meta: &mut VirtualCells<'_, F>,
        item: MLookupItem,
        common_config: &EventTableCommonConfig<F>,
    ) -> Option<Expression<F>> {
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
                constant_from!(VarType::I64),
                self.load_value1.expr(meta),
            )),
            MLookupItem::Third => Some(
                MemoryTableLookupEncode::encode_memory_load(
                    common_config.eid(meta),
                    constant_from!(3),
                    common_config.mmid(meta),
                    self.load_end_block_index.expr(meta),
                    constant_from!(VarType::I64),
                    self.load_value2.expr(meta),
                ) * cross_load.clone(),
            ),
            MLookupItem::Fourth => Some(MemoryTableLookupEncode::encode_stack_write(
                common_config.eid(meta),
                constant_from!(3) + cross_load,
                common_config.sp(meta) + constant_from!(1),
                self.vtype.expr(meta),
                self.res.expr(meta),
            )),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::test::test_circuit_noexternal;

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

    #[test]
    fn test_load_64() {
        let textual_repr = r#"
                (module
                    (memory $0 1)
                    (data (i32.const 0) "\ff\00\00\00\fe\00\00\00")
                    (func (export "test")
                      (i32.const 0)
                      (i64.load offset=0)
                      (drop)
                      (i32.const 4)
                      (i64.load offset=4)
                      (drop)
                    )
                   )
                "#;

        test_circuit_noexternal(textual_repr).unwrap();
    }

    #[test]
    fn test_load_32_8s() {
        let textual_repr = r#"
                (module
                    (memory $0 1)
                    (data (i32.const 0) "\ff\00\00\00\fe\00\00\00")
                    (func (export "test")
                      (i32.const 0)
                      (i32.load8_s offset=0)
                      (drop)
                    )
                   )
                "#;

        test_circuit_noexternal(textual_repr).unwrap();
    }

    #[test]
    fn test_load_64_8u() {
        let textual_repr = r#"
                (module
                    (memory $0 1)
                    (data (i32.const 0) "\ff\00\00\00\fe\00\00\00")
                    (func (export "test")
                      (i32.const 0)
                      (i64.load8_u offset=0)
                      (drop)
                    )
                   )
                "#;

        test_circuit_noexternal(textual_repr).unwrap();
    }

    #[test]
    fn test_load_64_cross() {
        let textual_repr = r#"
                (module
                    (memory $0 1)
                    (data (i32.const 0) "\ff\00\00\00\fe\00\00\00\fd\00\00\00\fc\00\00\00")
                    (func (export "test")
                      (i32.const 4)
                      (i64.load offset=0)
                      (drop)
                      (i32.const 6)
                      (i64.load32_u offset=0)
                      (drop)
                      (i32.const 7)
                      (i64.load16_u offset=0)
                      (drop)
                    )
                   )
                "#;

        test_circuit_noexternal(textual_repr).unwrap();
    }

    #[test]
    fn test_all_load() {
        let textual_repr = r#"
                (module
                    (memory $0 1)
                    (data (i32.const 0) "\ff\00\00\00\fe\00\00\00")
                    (func (export "test")
                        (i32.const 0)
                        (i64.load offset=0)
                        (drop)
                        (i32.const 0)
                        (i64.load32_u offset=0)
                        (drop)
                        (i32.const 0)
                        (i64.load32_s offset=0)
                        (drop)
                        (i32.const 0)
                        (i64.load16_u offset=0)
                        (drop)
                        (i32.const 0)
                        (i64.load16_s offset=0)
                        (drop)
                        (i32.const 0)
                        (i64.load8_u offset=0)
                        (drop)
                        (i32.const 0)
                        (i64.load8_s offset=0)
                        (drop)

                        (i32.const 0)
                        (i32.load offset=0)
                        (drop)
                        (i32.const 0)
                        (i32.load16_u offset=0)
                        (drop)
                        (i32.const 0)
                        (i32.load16_s offset=0)
                        (drop)
                        (i32.const 0)
                        (i32.load8_u offset=0)
                        (drop)
                        (i32.const 0)
                        (i32.load8_s offset=0)
                        (drop)
                    )
                )
                "#;

        test_circuit_noexternal(textual_repr).unwrap();
    }
}
