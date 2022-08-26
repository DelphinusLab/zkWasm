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

pub struct LoadConfig<F: FieldExt> {
    load_base: U32Config<F>,
    load_offset: U32Config<F>,

    // bytes8_offset + bytes8_address * 8 = load_base + load_offset
    // offset range also limited by configure_in_byte_shift_range
    bytes8_offset: U8Config<F>,
    bytes8_address: U32Config<F>,
    bytes8_value: Bytes8Config<F>,

    final_bytes_shifts: [Column<Advice>; 8],
    final_value: Column<Advice>,
    // final_vtype range also limited by configure_in_byte_shift_range
    final_vtype: U8Config<F>,

    enable: Column<Advice>,
    _mark: PhantomData<F>,
}

pub struct LoadConfigBuilder {}

impl<F: FieldExt> EventTableOpcodeConfigBuilder<F> for LoadConfigBuilder {
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

        let load_base = U32Config::configure(meta, cols, rtable, &enable_fn);
        let load_offset = U32Config::configure(meta, cols, rtable, &enable_fn);
        let bytes8_address = U32Config::configure(meta, cols, rtable, &enable_fn);
        let bytes8_offset = U8Config::configure(meta, cols, rtable, &enable_fn);
        let bytes8_value = Bytes8Config::configure(meta, cols, rtable, &enable_fn);

        let final_bytes_shifts = [0; 8].map(|_| cols.next().unwrap());
        let final_value = cols.next().unwrap();
        let final_vtype = U8Config::configure(meta, cols, rtable, &enable_fn);

        mtable.configure_stack_read_in_table(
            "op_load get load_base",
            meta,
            &enable_fn,
            |meta| curr!(meta, common.eid),
            |_meta| constant_from!(1u64),
            |meta| curr!(meta, common.sp) + constant_from!(1),
            |_meta| constant_from!(VarType::I32),
            |meta| curr!(meta, load_base.value),
        );

        mtable.configure_memory_load_in_table(
            "op_load read heap",
            meta,
            &enable_fn,
            |meta| curr!(meta, common.eid),
            |_meta| constant_from!(2u64),
            |meta| curr!(meta, common.mmid),
            |meta| curr!(meta, bytes8_address.value),
            |_meta| constant_from!(VarType::U64),
            |meta| curr!(meta, bytes8_value.value),
        );

        mtable.configure_stack_write_in_table(
            "op_load push value to stack",
            meta,
            &enable_fn,
            |meta| curr!(meta, common.eid),
            |_meta| constant_from!(3u64),
            |meta| curr!(meta, common.sp) + constant_from!(1),
            |meta| curr!(meta, final_vtype.value),
            |meta| curr!(meta, final_value),
        );

        meta.create_gate("op_load address equation", |meta| {
            vec![
                (curr!(meta, load_base.value) + curr!(meta, load_offset.value)
                    - curr!(meta, bytes8_address.value) * constant_from!(8)
                    - curr!(meta, bytes8_offset.value))
                    * enable_fn(meta),
            ]
        });

        for i in 0..8 {
            rtable.configure_in_byte_shift_range(
                meta,
                "op_load bytes shift",
                |meta| {
                    (
                        constant_from!(i),
                        curr!(meta, final_vtype.value),
                        curr!(meta, bytes8_offset.value),
                        curr!(meta, bytes8_value.bytes_le[i]),
                        curr!(meta, final_bytes_shifts[i]),
                    )
                },
                &enable_fn,
            )
        }

        meta.create_gate("op_load final value equation", |meta| {
            let acc = final_bytes_shifts
                .iter()
                .map(|col| curr!(meta, *col))
                .reduce(|acc, v| acc + v)
                .unwrap();

            vec![(acc - curr!(meta, final_value)) * enable_fn(meta)]
        });

        Box::new(LoadConfig {
            enable: opcode_bit,
            _mark: PhantomData,
            load_base,
            load_offset,
            bytes8_offset,
            bytes8_address,
            bytes8_value,
            final_bytes_shifts,
            final_value,
            final_vtype,
        })
    }
}

impl<F: FieldExt> EventTableOpcodeConfig<F> for LoadConfig<F> {
    fn opcode(&self, meta: &mut VirtualCells<'_, F>) -> Expression<F> {
        (constant!(bn_to_field(
            &(BigUint::from(OpcodeClass::Load as u64) << OPCODE_CLASS_SHIFT)
        )) + curr!(meta, self.final_vtype.value)
            * constant!(bn_to_field(&(BigUint::from(1u64) << OPCODE_ARG0_SHIFT)))
            + curr!(meta, self.load_offset.value))
            * curr!(meta, self.enable)
    }

    fn sp_diff(&self, _meta: &mut VirtualCells<'_, F>) -> Expression<F> {
        constant!(F::zero())
    }

    fn opcode_class(&self) -> OpcodeClass {
        OpcodeClass::Load
    }

    fn assign(&self, ctx: &mut Context<'_, F>, entry: &EventTableEntry) -> Result<(), Error> {
        match entry.step_info {
            specs::step::StepInfo::Load {
                vtype,
                offset,
                value,
                block_value,
                raw_address,
                effective_address,
                ..
            } => {
                self.load_base.assign(ctx, raw_address.into())?;
                self.load_offset.assign(ctx, offset.into())?;

                let bytes8_offset = effective_address as u64 % 8;
                self.bytes8_address
                    .assign(ctx, effective_address as u64 / 8)?;
                self.bytes8_offset.assign(ctx, bytes8_offset)?;
                self.bytes8_value.assign(ctx, block_value)?;
                self.final_vtype.assign(ctx, vtype as u64)?;

                ctx.region.assign_advice(
                    || "op_load final_value",
                    self.final_value,
                    ctx.offset,
                    || Ok(F::from(value)),
                )?;

                let bytes = block_value.to_le_bytes();
                for i in 0..8 {
                    let value = byte_shift(vtype, bytes8_offset as usize, i, bytes[i] as u64);

                    ctx.region.assign_advice(
                        || "op_load final_bytes_shifts",
                        self.final_bytes_shifts[i],
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
    use crate::test::test_circuit_noexternal;

    #[test]
    fn test_load() {
        let textual_repr = r#"
                (module
                    (memory $0 1)
                    (data (i32.const 0) "\01\00\00\00\01\00\00\00")
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
