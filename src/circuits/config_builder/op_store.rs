use crate::{
    circuits::{
        etable::{EventTableCommonConfig, EventTableOpcodeConfig, EventTableOpcodeConfigBuilder},
        itable::InstructionTableConfig,
        jtable::JumpTableConfig,
        mtable::MemoryTableConfig,
        rtable::{byte_shift, RangeTableConfig},
        utils::{bn_to_field, bytes8::Bytes8Config, u32::U32Config, u8::U8Config, Context},
    },
    constant, constant_from, curr,
};
use halo2_proofs::{
    arithmetic::FieldExt,
    plonk::{Advice, Column, ConstraintSystem, Error, Expression, VirtualCells},
};
use num_bigint::BigUint;
use specs::{
    etable::EventTableEntry,
    itable::{OpcodeClass, OPCODE_ARG0_SHIFT, OPCODE_CLASS_SHIFT},
    mtable::VarType,
};
use std::marker::PhantomData;

pub struct StoreConfig<F: FieldExt> {
    store_base: U32Config<F>,
    store_offset: U32Config<F>,

    vtype: U8Config<F>,
    bytes_shifts: [Column<Advice>; 8],
    value: Column<Advice>,

    // bytes8_offset + bytes8_address * 8 = load_base + load_offset
    // offset range also limited by configure_in_byte_shift_range
    bytes8_offset: U8Config<F>,
    bytes8_address: U32Config<F>,
    bytes8_value_pre: Bytes8Config<F>,
    bytes8_value_post: Bytes8Config<F>,

    enable: Column<Advice>,
    _mark: PhantomData<F>,
}

pub struct StoreConfigBuilder {}

impl<F: FieldExt> EventTableOpcodeConfigBuilder<F> for StoreConfigBuilder {
    fn configure(
        meta: &mut ConstraintSystem<F>,
        common: &EventTableCommonConfig,
        opcode_bit: Column<Advice>,
        cols: &mut impl Iterator<Item = Column<Advice>>,
        rtable: &RangeTableConfig<F>,
        _itable: &InstructionTableConfig<F>,
        mtable: &MemoryTableConfig<F>,
        _jtable: &JumpTableConfig<F>,
        enable: impl Fn(&mut VirtualCells<'_, F>) -> Expression<F>,
    ) -> Box<dyn EventTableOpcodeConfig<F>> {
        let enable_fn = |meta: &mut VirtualCells<F>| curr!(meta, opcode_bit) * enable(meta);

        let store_base = U32Config::configure(meta, cols, rtable, &enable_fn);
        let store_offset = U32Config::configure(meta, cols, rtable, &enable_fn);
        let bytes8_address = U32Config::configure(meta, cols, rtable, &enable_fn);
        let bytes8_offset = U8Config::configure(meta, cols, rtable, &enable_fn);
        let bytes8_value_pre = Bytes8Config::configure(meta, cols, rtable, &enable_fn);
        let bytes8_value_post = Bytes8Config::configure(meta, cols, rtable, &enable_fn);

        let bytes_shifts = [0; 8].map(|_| cols.next().unwrap());
        let value = cols.next().unwrap();
        let vtype = U8Config::configure(meta, cols, rtable, &enable_fn);

        mtable.configure_stack_read_in_table(
            "op_store get value",
            meta,
            &enable_fn,
            |meta| curr!(meta, common.eid),
            |_meta| constant_from!(1u64),
            |meta| curr!(meta, common.sp) + constant_from!(1),
            |meta| curr!(meta, vtype.value),
            |meta| curr!(meta, value),
        );

        mtable.configure_stack_read_in_table(
            "op_store get load_base",
            meta,
            &enable_fn,
            |meta| curr!(meta, common.eid),
            |_meta| constant_from!(2u64),
            |meta| curr!(meta, common.sp) + constant_from!(2),
            |_meta| constant_from!(VarType::I32),
            |meta| curr!(meta, store_base.value),
        );

        mtable.configure_memory_load_in_table(
            "op_store read heap",
            meta,
            &enable_fn,
            |meta| curr!(meta, common.eid),
            |_meta| constant_from!(3u64),
            |meta| curr!(meta, common.mmid),
            |meta| curr!(meta, bytes8_address.value),
            |_meta| constant_from!(VarType::U64),
            |meta| curr!(meta, bytes8_value_pre.value),
        );

        mtable.configure_memory_store_in_table(
            "op_store read heap",
            meta,
            &enable_fn,
            |meta| curr!(meta, common.eid),
            |_meta| constant_from!(4u64),
            |meta| curr!(meta, common.mmid),
            |meta| curr!(meta, bytes8_address.value),
            |_meta| constant_from!(VarType::U64),
            |meta| curr!(meta, bytes8_value_post.value),
        );

        meta.create_gate("op_store address equation", |meta| {
            vec![
                (curr!(meta, store_base.value) + curr!(meta, store_offset.value)
                    - curr!(meta, bytes8_address.value) * constant_from!(8)
                    - curr!(meta, bytes8_offset.value))
                    * enable_fn(meta),
            ]
        });

        for i in 0..8 {
            rtable.configure_in_byte_shift_range(
                meta,
                "op_store bytes shift",
                |meta| {
                    (
                        constant_from!(i),
                        curr!(meta, vtype.value),
                        curr!(meta, bytes8_offset.value),
                        curr!(meta, bytes8_value_post.bytes_le[i]),
                        curr!(meta, bytes_shifts[i]),
                    )
                },
                &enable_fn,
            );

            rtable.configure_in_unchangable_range(
                meta,
                "op_store byte unchangable",
                |meta| {
                    (
                        constant_from!(i),
                        curr!(meta, vtype.value),
                        curr!(meta, bytes8_offset.value),
                        curr!(meta, bytes8_value_post.bytes_le[i])
                            - curr!(meta, bytes8_value_pre.bytes_le[i]),
                    )
                },
                &enable_fn,
            );
        }

        meta.create_gate("op_store final value equation", |meta| {
            let acc = bytes_shifts
                .iter()
                .map(|col| curr!(meta, *col))
                .reduce(|acc, v| acc + v)
                .unwrap();

            vec![(acc - curr!(meta, value)) * enable_fn(meta)]
        });

        Box::new(StoreConfig {
            enable: opcode_bit,
            _mark: PhantomData,
            store_base,
            store_offset,
            vtype,
            bytes_shifts,
            value,
            bytes8_offset,
            bytes8_address,
            bytes8_value_pre,
            bytes8_value_post,
        })
    }
}

impl<F: FieldExt> EventTableOpcodeConfig<F> for StoreConfig<F> {
    fn opcode(&self, meta: &mut VirtualCells<'_, F>) -> Expression<F> {
        (constant!(bn_to_field(
            &(BigUint::from(OpcodeClass::Store as u64) << OPCODE_CLASS_SHIFT)
        )) + curr!(meta, self.vtype.value)
            * constant!(bn_to_field(&(BigUint::from(1u64) << OPCODE_ARG0_SHIFT)))
            + curr!(meta, self.store_offset.value))
            * curr!(meta, self.enable)
    }

    fn sp_diff(&self, _meta: &mut VirtualCells<'_, F>) -> Expression<F> {
        constant!(F::from(2))
    }

    fn opcode_class(&self) -> OpcodeClass {
        OpcodeClass::Store
    }

    fn assign(&self, ctx: &mut Context<'_, F>, entry: &EventTableEntry) -> Result<(), Error> {
        match entry.step_info {
            specs::step::StepInfo::Store {
                vtype,
                offset,
                value,
                raw_address,
                effective_address,
                pre_block_value,
                updated_block_value,
                ..
            } => {
                self.store_base.assign(ctx, raw_address.into())?;
                self.store_offset.assign(ctx, offset.into())?;
                self.bytes8_address
                    .assign(ctx, effective_address as u64 / 8)?;
                self.bytes8_offset
                    .assign(ctx, effective_address as u64 % 8)?;
                self.bytes8_value_pre.assign(ctx, pre_block_value)?;
                self.bytes8_value_post.assign(ctx, updated_block_value)?;
                self.vtype.assign(ctx, vtype as u64)?;

                ctx.region.assign_advice(
                    || "op_store final_value",
                    self.value,
                    ctx.offset,
                    || Ok(F::from(value)),
                )?;

                let bytes = updated_block_value.to_le_bytes();
                for i in 0..8 {
                    let value = byte_shift(vtype, offset as usize, i, bytes[i] as u64);

                    ctx.region.assign_advice(
                        || "op_store final_bytes_shifts",
                        self.bytes_shifts[i],
                        ctx.offset,
                        || Ok(value.into()),
                    )?;
                }
            }
            _ => unreachable!(),
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        runtime::{WasmInterpreter, WasmRuntime},
        test::test_circuit_builder::run_test_circuit,
    };
    use halo2_proofs::pairing::bn256::Fr as Fp;
    use wasmi::{ImportsBuilder, NopExternals};

    #[test]
    fn test_store() {
        let textual_repr = r#"
                (module
                    (memory $0 1)
                    (data (i32.const 0) "\01\00\00\00\01\00\00\00")
                    (func (export "test")
                      (i32.const 0)
                      (i32.const 2)
                      (i32.store offset=0)
                    )
                   )
                "#;

        let compiler = WasmInterpreter::new();
        let compiled_module = compiler
            .compile(textual_repr, &ImportsBuilder::default())
            .unwrap();
        let execution_log = compiler
            .run(&mut NopExternals, &compiled_module, "test", vec![])
            .unwrap();

        run_test_circuit::<Fp>(compiled_module.tables, execution_log.tables).unwrap()
    }
}
