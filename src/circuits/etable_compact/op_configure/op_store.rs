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
    itable::{OpcodeClass, OPCODE_ARG0_SHIFT, OPCODE_ARG1_SHIFT, OPCODE_CLASS_SHIFT},
};
use specs::{mtable::VarType, step::StepInfo};

pub struct StoreConfig {
    opcode_store_offset: CommonRangeCell,

    store_start_block_index: CommonRangeCell,
    store_start_block_inner_offset: CommonRangeCell,
    store_start_block_inner_offset_helper: CommonRangeCell,

    store_end_block_index: CommonRangeCell,
    store_end_block_inner_offset: CommonRangeCell,
    store_end_block_inner_offset_helper: CommonRangeCell,

    load_value1: U64OnU8Cell,
    load_value2: U64OnU8Cell,
    store_value1: U64OnU8Cell,
    store_value2: U64OnU8Cell,

    mask_bits: [BitCell; 16],
    offset_modulus: U64Cell,
    store_raw_value: U64Cell,
    store_base: U64Cell,
    store_wrapped_value: UnlimitedCell,

    vtype: CommonRangeCell,
    is_one_byte: BitCell,
    is_two_bytes: BitCell,
    is_four_bytes: BitCell,
    is_eight_bytes: BitCell,

    lookup_stack_read_pos: MTableLookupCell,
    lookup_stack_read_val: MTableLookupCell,
    lookup_heap_read1: MTableLookupCell,
    _lookup_heap_read2: MTableLookupCell,
    lookup_heap_write1: MTableLookupCell,
    _lookup_heap_write2: MTableLookupCell,

    lookup_offset_len_bits: OffsetLenBitsTableLookupCell,
    lookup_pow: PowTableLookupCell,
}

pub struct StoreConfigBuilder {}

impl<F: FieldExt> EventTableOpcodeConfigBuilder<F> for StoreConfigBuilder {
    fn configure(
        common: &mut EventTableCellAllocator<F>,
        constraint_builder: &mut ConstraintBuilder<F>,
    ) -> Box<dyn EventTableOpcodeConfig<F>> {
        let opcode_store_offset = common.alloc_common_range_value();

        let store_start_block_index = common.alloc_common_range_value();
        let store_start_block_inner_offset = common.alloc_common_range_value();
        let store_start_block_inner_offset_helper = common.alloc_common_range_value();

        let store_end_block_index = common.alloc_common_range_value();
        let store_end_block_inner_offset = common.alloc_common_range_value();
        let store_end_block_inner_offset_helper = common.alloc_common_range_value();

        let load_value1 = common.alloc_u64_on_u8();
        let load_value2 = common.alloc_u64_on_u8();
        let store_value1 = common.alloc_u64_on_u8();
        let store_value2 = common.alloc_u64_on_u8();
        let offset_modulus = common.alloc_u64();
        let store_raw_value = common.alloc_u64();
        let store_base = common.alloc_u64();

        let store_wrapped_value = common.alloc_unlimited_value();

        let mask_bits = [0; 16].map(|_| common.alloc_bit_value());
        let is_one_byte = common.alloc_bit_value();
        let is_two_bytes = common.alloc_bit_value();
        let is_four_bytes = common.alloc_bit_value();
        let is_eight_bytes = common.alloc_bit_value();
        let vtype = common.alloc_common_range_value();

        let lookup_stack_read_val = common.alloc_mtable_lookup();
        let lookup_stack_read_pos = common.alloc_mtable_lookup();
        let lookup_heap_read1 = common.alloc_mtable_lookup();
        let lookup_heap_read2 = common.alloc_mtable_lookup();
        let lookup_heap_write1 = common.alloc_mtable_lookup();
        let lookup_heap_write2 = common.alloc_mtable_lookup();

        let lookup_offset_len_bits = common.alloc_offset_len_bits_table_lookup();
        let lookup_pow = common.alloc_pow_table_lookup();

        constraint_builder.push(
            "op_store start end offset range",
            Box::new(move |meta| {
                vec![
                    store_start_block_inner_offset.expr(meta)
                        + store_start_block_inner_offset_helper.expr(meta)
                        - constant_from!(7),
                    store_end_block_inner_offset.expr(meta)
                        + store_end_block_inner_offset_helper.expr(meta)
                        - constant_from!(7),
                ]
            }),
        );

        constraint_builder.push(
            "op_store start end equation",
            Box::new(move |meta| {
                let len = constant_from!(1)
                    + is_two_bytes.expr(meta) * constant_from!(1)
                    + is_four_bytes.expr(meta) * constant_from!(3)
                    + is_eight_bytes.expr(meta) * constant_from!(7);
                vec![
                    store_start_block_index.expr(meta) * constant_from!(8)
                        + store_start_block_inner_offset.expr(meta)
                        + len
                        - constant_from!(1)
                        - store_end_block_index.expr(meta) * constant_from!(8)
                        - store_end_block_inner_offset.expr(meta),
                ]
            }),
        );

        constraint_builder.push(
            "op_store start store_base",
            Box::new(move |meta| {
                vec![
                    store_base.expr(meta) + opcode_store_offset.expr(meta)
                        - store_start_block_index.expr(meta) * constant_from!(8)
                        - store_start_block_inner_offset.expr(meta),
                ]
            }),
        );

        constraint_builder.push(
            "op_store length",
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
            "op_store mask_bits offset len",
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
                            store_start_block_inner_offset.expr(meta),
                            len,
                            bits_encode,
                        ),
                ]
            }),
        );

        constraint_builder.push(
            "op_store pow table lookup",
            Box::new(move |meta| {
                vec![
                    lookup_pow.expr(meta)
                        - pow_table_encode(
                            offset_modulus.expr(meta),
                            store_start_block_inner_offset.expr(meta) * constant_from!(8),
                        ),
                ]
            }),
        );

        constraint_builder.push(
            "op_store wrap value",
            Box::new(move |meta| {
                let has_two_bytes =
                    is_two_bytes.expr(meta) + is_four_bytes.expr(meta) + is_eight_bytes.expr(meta);
                let has_four_bytes = is_four_bytes.expr(meta) + is_eight_bytes.expr(meta);
                let has_eight_bytes = is_eight_bytes.expr(meta);

                let byte_value = (0..8)
                    .map(|i| {
                        store_raw_value.u4_expr(meta, i * 2) * constant_from!(1u64 << (8 * i))
                            + store_raw_value.u4_expr(meta, i * 2 + 1)
                                * constant_from!(1u64 << (8 * i + 4))
                    })
                    .collect::<Vec<_>>();

                vec![
                    byte_value[0].clone()
                        + byte_value[1].clone() * has_two_bytes
                        + (byte_value[2].clone() + byte_value[3].clone()) * has_four_bytes
                        + (byte_value[4].clone()
                            + byte_value[5].clone()
                            + byte_value[6].clone()
                            + byte_value[7].clone())
                            * has_eight_bytes
                        - store_wrapped_value.expr(meta),
                ]
            }),
        );

        constraint_builder.push(
            "op_store write value",
            Box::new(move |meta| {
                let mut acc = store_wrapped_value.expr(meta) * offset_modulus.expr(meta);

                for i in 0..8 {
                    acc = acc
                        - store_value1.u8_expr(meta, i)
                            * constant!(bn_to_field(&(BigUint::from(1u64) << (i * 8))))
                            * mask_bits[i as usize].expr(meta);

                    acc = acc
                        - store_value2.u8_expr(meta, i)
                            * constant!(bn_to_field(&(BigUint::from(1u64) << (i * 8 + 64))))
                            * mask_bits[i as usize + 8].expr(meta);
                }

                vec![acc]
            }),
        );

        constraint_builder.push(
            "op_store unchanged value",
            Box::new(move |meta| {
                let mut acc = constant_from!(0);

                for i in 0..8 {
                    acc = acc
                        + load_value1.u8_expr(meta, i)
                            * constant!(bn_to_field(&(BigUint::from(1u64) << (i * 8))))
                            * (constant_from!(1) - mask_bits[i as usize].expr(meta))
                        - store_value1.u8_expr(meta, i)
                            * constant!(bn_to_field(&(BigUint::from(1u64) << (i * 8))))
                            * (constant_from!(1) - mask_bits[i as usize].expr(meta));

                    acc = acc
                        + load_value2.u8_expr(meta, i)
                            * constant!(bn_to_field(&(BigUint::from(1u64) << (i * 8 + 64))))
                            * mask_bits[i as usize + 8].expr(meta)
                        - store_value2.u8_expr(meta, i)
                            * constant!(bn_to_field(&(BigUint::from(1u64) << (i * 8 + 64))))
                            * mask_bits[i as usize + 8].expr(meta);
                }

                vec![acc]
            }),
        );

        Box::new(StoreConfig {
            opcode_store_offset,
            store_start_block_index,
            store_start_block_inner_offset,
            store_start_block_inner_offset_helper,
            store_end_block_index,
            store_end_block_inner_offset,
            store_end_block_inner_offset_helper,
            store_value1,
            store_value2,
            mask_bits,
            offset_modulus,
            store_base,
            store_raw_value,
            store_wrapped_value,
            is_one_byte,
            is_two_bytes,
            is_four_bytes,
            is_eight_bytes,
            vtype,
            lookup_heap_read1,
            _lookup_heap_read2: lookup_heap_read2,
            lookup_offset_len_bits,
            lookup_pow,
            lookup_stack_read_pos,
            lookup_stack_read_val,
            lookup_heap_write1,
            _lookup_heap_write2: lookup_heap_write2,
            load_value1,
            load_value2,
        })
    }
}

impl<F: FieldExt> EventTableOpcodeConfig<F> for StoreConfig {
    fn opcode(&self, meta: &mut VirtualCells<'_, F>) -> Expression<F> {
        let store_size = self.is_eight_bytes.expr(meta) * constant_from!(3)
            + self.is_four_bytes.expr(meta) * constant_from!(2)
            + self.is_two_bytes.expr(meta) * constant_from!(1)
            + constant_from!(1);

        constant!(bn_to_field(
            &(BigUint::from(OpcodeClass::Store as u64) << OPCODE_CLASS_SHIFT)
        )) + self.vtype.expr(meta)
            * constant!(bn_to_field(&(BigUint::from(1u64) << OPCODE_ARG0_SHIFT)))
            + store_size * constant!(bn_to_field(&(BigUint::from(1u64) << OPCODE_ARG1_SHIFT)))
            + self.opcode_store_offset.expr(meta)
    }

    fn assign(
        &self,
        ctx: &mut Context<'_, F>,
        step_info: &StepStatus,
        entry: &EventTableEntry,
    ) -> Result<(), Error> {
        match entry.step_info {
            StepInfo::Store {
                vtype,
                store_size,
                offset,
                raw_address,
                effective_address,
                pre_block_value,
                updated_block_value,
                value,
                mmid,
            } => {
                self.opcode_store_offset
                    .assign(ctx, F::from(offset as u64))?;

                let len = store_size.byte_size();

                let start_byte_index = effective_address as u64;
                let end_byte_index = start_byte_index + len - 1;

                self.store_start_block_index
                    .assign(ctx, (start_byte_index / 8).try_into().unwrap())?;
                self.store_start_block_inner_offset
                    .assign(ctx, (start_byte_index % 8).try_into().unwrap())?;
                self.store_start_block_inner_offset_helper
                    .assign(ctx, (7 - start_byte_index % 8).try_into().unwrap())?;

                self.store_end_block_index
                    .assign(ctx, (end_byte_index / 8).try_into().unwrap())?;
                self.store_end_block_inner_offset
                    .assign(ctx, (end_byte_index % 8).try_into().unwrap())?;
                self.store_end_block_inner_offset_helper
                    .assign(ctx, (7 - end_byte_index % 8).try_into().unwrap())?;

                self.load_value1.assign(ctx, pre_block_value)?;
                self.store_value1.assign(ctx, updated_block_value)?;

                // TODO replace 0 if cross store
                self.load_value2.assign(ctx, 0)?;
                self.store_value2.assign(ctx, 0)?;

                let offset = start_byte_index % 8;
                let bits = bits_of_offset_len(offset, len);
                for i in 0..16 {
                    self.mask_bits[i].assign(ctx, (bits >> i) & 1 == 1)?;
                }
                self.offset_modulus.assign(ctx, 1 << (offset * 8))?;
                self.store_raw_value.assign(ctx, value)?;
                self.store_base.assign(ctx, raw_address.into())?;
                self.store_wrapped_value.assign(
                    ctx,
                    F::from(if store_size.byte_size() == 8 {
                        value
                    } else {
                        value & ((1u64 << (store_size.byte_size() * 8)) - 1)
                    }),
                )?;

                self.is_one_byte.assign(ctx, len == 1)?;
                self.is_two_bytes.assign(ctx, len == 2)?;
                self.is_four_bytes.assign(ctx, len == 4)?;
                self.is_eight_bytes.assign(ctx, len == 8)?;
                self.vtype.assign(ctx, F::from(vtype as u64))?;

                self.lookup_stack_read_val.assign(
                    ctx,
                    &MemoryTableLookupEncode::encode_stack_read(
                        BigUint::from(step_info.current.eid),
                        BigUint::from(1 as u64),
                        BigUint::from(step_info.current.sp + 1),
                        BigUint::from(vtype as u64),
                        BigUint::from(value),
                    ),
                )?;

                self.lookup_stack_read_pos.assign(
                    ctx,
                    &MemoryTableLookupEncode::encode_stack_read(
                        BigUint::from(step_info.current.eid),
                        BigUint::from(2 as u64),
                        BigUint::from(step_info.current.sp + 2),
                        BigUint::from(VarType::I32 as u16),
                        BigUint::from(raw_address),
                    ),
                )?;

                self.lookup_heap_read1.assign(
                    ctx,
                    &MemoryTableLookupEncode::encode_memory_load(
                        BigUint::from(step_info.current.eid),
                        BigUint::from(3 as u64),
                        BigUint::from(mmid),
                        BigUint::from(start_byte_index / 8),
                        BigUint::from(VarType::I64 as u16),
                        BigUint::from(pre_block_value),
                    ),
                )?;

                self.lookup_heap_write1.assign(
                    ctx,
                    &MemoryTableLookupEncode::encode_memory_store(
                        BigUint::from(step_info.current.eid),
                        BigUint::from(4 as u64),
                        BigUint::from(mmid),
                        BigUint::from(start_byte_index / 8),
                        BigUint::from(VarType::I64 as u16),
                        BigUint::from(updated_block_value),
                    ),
                )?;

                //TODO: assign for cross store block value

                self.lookup_offset_len_bits.assign(ctx, offset, len)?;
                self.lookup_pow.assign(ctx, offset * 8)?;

                Ok(())
            }

            _ => unreachable!(),
        }
    }

    fn sp_diff(&self, _meta: &mut VirtualCells<'_, F>) -> Option<Expression<F>> {
        Some(constant_from!(2))
    }

    fn opcode_class(&self) -> OpcodeClass {
        OpcodeClass::Store
    }

    fn mops(&self, meta: &mut VirtualCells<'_, F>) -> Option<Expression<F>> {
        let cross_block =
            self.store_end_block_index.expr(meta) - self.store_start_block_index.expr(meta);
        Some(constant_from!(4) + cross_block * constant_from!(2))
    }

    fn assigned_extra_mops(
        &self,
        _ctx: &mut Context<'_, F>,
        _step: &StepStatus,
        entry: &EventTableEntry,
    ) -> u64 {
        match &entry.step_info {
            StepInfo::Store {
                store_size,
                effective_address,
                ..
            } => {
                if (*effective_address + store_size.byte_size() as u32 - 1) / 8
                    != *effective_address / 8
                {
                    2
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
        let cross_block =
            self.store_end_block_index.expr(meta) - self.store_start_block_index.expr(meta);

        match item {
            MLookupItem::First => Some(MemoryTableLookupEncode::encode_stack_read(
                common_config.eid(meta),
                constant_from!(1),
                common_config.sp(meta) + constant_from!(1),
                self.vtype.expr(meta),
                self.store_raw_value.expr(meta),
            )),
            MLookupItem::Second => Some(MemoryTableLookupEncode::encode_stack_read(
                common_config.eid(meta),
                constant_from!(2),
                common_config.sp(meta) + constant_from!(2),
                constant_from!(VarType::I32),
                self.store_base.expr(meta),
            )),
            MLookupItem::Third => Some(MemoryTableLookupEncode::encode_memory_load(
                common_config.eid(meta),
                constant_from!(3),
                common_config.mmid(meta),
                self.store_start_block_index.expr(meta),
                constant_from!(VarType::I64),
                self.load_value1.expr(meta),
            )),
            MLookupItem::Fourth => Some(
                MemoryTableLookupEncode::encode_memory_load(
                    common_config.eid(meta),
                    constant_from!(4),
                    common_config.mmid(meta),
                    self.store_end_block_index.expr(meta),
                    constant_from!(VarType::I64),
                    self.load_value2.expr(meta),
                ) * cross_block.clone(),
            ),
            MLookupItem::Fifth => Some(MemoryTableLookupEncode::encode_memory_store(
                common_config.eid(meta),
                constant_from!(4) + cross_block,
                common_config.mmid(meta),
                self.store_start_block_index.expr(meta),
                constant_from!(VarType::I64),
                self.store_value1.expr(meta),
            )),
            MLookupItem::Six => Some(
                MemoryTableLookupEncode::encode_memory_store(
                    common_config.eid(meta),
                    constant_from!(6),
                    common_config.mmid(meta),
                    self.store_end_block_index.expr(meta),
                    constant_from!(VarType::I64),
                    self.store_value2.expr(meta),
                ) * cross_block.clone(),
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::test::test_circuit_noexternal;

    #[test]
    fn test_store_32() {
        let textual_repr = r#"
                (module
                    (memory $0 1)
                    (data (i32.const 0) "\ff\00\00\00\fe\00\00\00")
                    (func (export "test")
                      (i32.const 1)
                      (i32.const 0)
                      (i32.store offset=0)
                      (i32.const 2)
                      (i32.const 4)
                      (i32.store offset=0)
                    )
                   )
                "#;

        test_circuit_noexternal(textual_repr).unwrap();
    }

    #[test]
    fn test_all_store() {
        let textual_repr = r#"
                (module
                    (memory $0 1)
                    (data (i32.const 0) "\ff\00\00\00\fe\00\00\00")
                    (func (export "test")
                      (i32.const 0)
                      (i64.const 0)
                      (i64.store offset=0)
                      (i32.const 0)
                      (i64.const 0)
                      (i64.store32 offset=0)
                      (i32.const 0)
                      (i64.const 0)
                      (i64.store16 offset=0)
                      (i32.const 0)
                      (i64.const 0)
                      (i64.store8 offset=0)

                      (i32.const 0)
                      (i32.const 0)
                      (i32.store offset=0)
                      (i32.const 0)
                      (i32.const 0)
                      (i32.store16 offset=0)
                      (i32.const 0)
                      (i32.const 0)
                      (i32.store8 offset=0)
                    )
                   )
                "#;

        test_circuit_noexternal(textual_repr).unwrap();
    }

    #[test]
    fn test_store_32_wrap() {
        let textual_repr = r#"
                (module
                    (memory $0 1)
                    (data (i32.const 0) "\ff\00\00\00\fe\00\00\00")
                    (func (export "test")
                      (i32.const 0)
                      (i32.const 256)
                      (i32.store8 offset=0)
                    )
                   )
                "#;

        test_circuit_noexternal(textual_repr).unwrap();
    }

    #[test]
    fn test_store_64() {
        let textual_repr = r#"
                (module
                    (memory $0 1)
                    (data (i32.const 0) "\ff\00\00\00\fe\00\00\00")
                    (func (export "test")
                      (i32.const 0)
                      (i64.const 0x432134214)
                      (i64.store offset=0)
                    )
                   )
                "#;

        test_circuit_noexternal(textual_repr).unwrap();
    }

    #[test]
    fn test_store_64_cross() {
        let textual_repr = r#"
                (module
                    (memory $0 1)
                    (data (i32.const 0) "\ff\00\00\00\fe\00\00\00")
                    (func (export "test")
                      (i32.const 6)
                      (i64.const 0x432134214)
                      (i64.store offset=0)
                    )
                   )
                "#;

        test_circuit_noexternal(textual_repr).unwrap();
    }
}
