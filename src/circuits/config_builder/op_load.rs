use crate::{
    circuits::{
        etable::{EventTableCommonConfig, EventTableOpcodeConfig, EventTableOpcodeConfigBuilder},
        itable::InstructionTableConfig,
        jtable::JumpTableConfig,
        mtable::MemoryTableConfig,
        rtable::RangeTableConfig,
        utils::{bn_to_field, tvalue::TValueConfig, u64::U64Config, Context},
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
    offset: Column<Advice>,
    raw_address: Column<Advice>,
    effective_address: TValueConfig<F>,
    value: U64Config<F>,
    // Used to lookup within imtable
    block_value: U64Config<F>,
    vtype: Column<Advice>,
    mmid: Column<Advice>,
    // Effective address div 8 to meet imtable's unit
    block_effective_address: Column<Advice>,
    position: Column<Advice>,
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
        let effective_address = TValueConfig::configure(meta, cols, rtable, |meta| {
            curr!(meta, opcode_bit) * enable(meta)
        });
        let block_effective_address = cols.next().unwrap();
        let position = cols.next().unwrap();
        let offset = cols.next().unwrap();
        let vtype = cols.next().unwrap();
        let raw_address = cols.next().unwrap();
        let mmid = cols.next().unwrap();
        let value = U64Config::configure(meta, cols, rtable, |meta| {
            curr!(meta, opcode_bit) * enable(meta)
        });
        let block_value = U64Config::configure(meta, cols, rtable, |meta| {
            curr!(meta, opcode_bit) * enable(meta)
        });

        rtable.configure_in_common_range(meta, "load offset range", |meta| {
            curr!(meta, opcode_bit) * curr!(meta, offset) * enable(meta)
        });

        // TODO: position should in range 0..8

        mtable.configure_stack_read_in_table(
            "op_load get raw_address",
            meta,
            |meta| curr!(meta, opcode_bit) * enable(meta),
            |meta| curr!(meta, common.eid),
            |_meta| constant_from!(1u64),
            |meta| curr!(meta, common.sp) + constant_from!(1),
            |_meta| constant_from!(VarType::I32),
            |meta| curr!(meta, raw_address),
        );

        mtable.configure_memory_load_in_table(
            "op_load get raw_address",
            meta,
            |meta| curr!(meta, opcode_bit) * enable(meta),
            |meta| curr!(meta, common.eid),
            |_meta| constant_from!(2u64),
            |meta| curr!(meta, common.mmid),
            |meta| curr!(meta, block_effective_address),
            |_meta| constant_from!(VarType::U64),
            |meta| curr!(meta, block_value.value),
        );

        mtable.configure_stack_write_in_table(
            "op_load push value to stack",
            meta,
            |meta| curr!(meta, opcode_bit) * enable(meta),
            |meta| curr!(meta, common.eid),
            |_meta| constant_from!(3u64),
            |meta| curr!(meta, common.sp),
            |meta| curr!(meta, vtype),
            |meta| curr!(meta, value.value),
        );

        meta.create_gate("effective_address equals offset plus raw_address", |meta| {
            vec![
                (curr!(meta, raw_address) + curr!(meta, offset)
                    - (curr!(meta, effective_address.value.value)))
                    * curr!(meta, opcode_bit)
                    * enable(meta),
                (curr!(meta, block_effective_address) * constant_from!(8) + curr!(meta, position)
                    - curr!(meta, effective_address.value.value))
                    * curr!(meta, opcode_bit)
                    * enable(meta),
            ]
        });

        Box::new(LoadConfig {
            offset,
            mmid,
            value,
            block_value,
            vtype,
            raw_address,
            effective_address,
            block_effective_address,
            position,
            enable: opcode_bit,
            _mark: PhantomData,
        })
    }
}

impl<F: FieldExt> EventTableOpcodeConfig<F> for LoadConfig<F> {
    fn opcode(&self, meta: &mut VirtualCells<'_, F>) -> Expression<F> {
        (constant!(bn_to_field(
            &(BigUint::from(OpcodeClass::Load as u64) << OPCODE_CLASS_SHIFT)
        )) + curr!(meta, self.vtype)
            * constant!(bn_to_field(&(BigUint::from(1u64) << OPCODE_ARG0_SHIFT)))
            + curr!(meta, self.offset))
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
                mmid,
            } => {
                ctx.region.assign_advice(
                    || "op_load offset",
                    self.offset,
                    ctx.offset,
                    || Ok(F::from(offset as u64)),
                )?;

                ctx.region.assign_advice(
                    || "op_load vtype",
                    self.vtype,
                    ctx.offset,
                    || Ok(F::from(vtype as u64)),
                )?;

                ctx.region.assign_advice(
                    || "op_load raw_address",
                    self.raw_address,
                    ctx.offset,
                    || Ok(F::from(raw_address as u64)),
                )?;

                self.effective_address
                    .assign(ctx, VarType::U32, (effective_address) as u64)?;

                ctx.region.assign_advice(
                    || "op_load mmid",
                    self.mmid,
                    ctx.offset,
                    || Ok(F::from(mmid)),
                )?;

                self.value.assign(ctx, value)?;
                self.block_value.assign(ctx, block_value)?;

                ctx.region.assign_advice(
                    || "op_load position",
                    self.position,
                    ctx.offset,
                    || Ok(F::from((effective_address % 8) as u64)),
                )?;

                ctx.region.assign_advice(
                    || "op_load block_effective_address",
                    self.block_effective_address,
                    ctx.offset,
                    || Ok(F::from((effective_address / 8) as u64)),
                )?;
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
                    )
                   )
                "#;

        let compiler = WasmInterpreter::new();
        let compiled_module = compiler.compile(textual_repr).unwrap();
        let execution_log = compiler.run(&compiled_module, "test", vec![]).unwrap();

        run_test_circuit::<Fp>(compiled_module.tables, execution_log.tables).unwrap()
    }
}
