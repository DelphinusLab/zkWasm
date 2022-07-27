use crate::{
    circuits::{
        etable::{EventTableCommonConfig, EventTableOpcodeConfig, EventTableOpcodeConfigBuilder},
        itable::InstructionTableConfig,
        jtable::JumpTableConfig,
        mtable::MemoryTableConfig,
        rtable::RangeTableConfig,
        utils::{bn_to_field, u64::U64Config, Context},
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
};
use std::marker::PhantomData;

pub struct LocalTeeConfig<F: FieldExt> {
    offset: Column<Advice>,
    value: U64Config<F>,
    vtype: Column<Advice>,
    enable: Column<Advice>,
    _mark: PhantomData<F>,
}

pub struct LocalTeeConfigBuilder {}

impl<F: FieldExt> EventTableOpcodeConfigBuilder<F> for LocalTeeConfigBuilder {
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
        let offset = cols.next().unwrap();
        let vtype = cols.next().unwrap();
        let value = U64Config::configure(meta, cols, rtable, |meta| {
            curr!(meta, opcode_bit) * enable(meta)
        });

        rtable.configure_in_common_range(meta, "localtee offset range", |meta| {
            curr!(meta, opcode_bit) * curr!(meta, offset) * enable(meta)
        });

        mtable.configure_stack_read_in_table(
            "local tee mlookup 1",
            meta,
            |meta| curr!(meta, opcode_bit) * enable(meta),
            |meta| curr!(meta, common.eid),
            |_meta| constant_from!(1u64),
            |meta| curr!(meta, common.sp) + constant_from!(1),
            |meta| curr!(meta, vtype),
            |meta| curr!(meta, value.value),
        );

        mtable.configure_stack_write_in_table(
            "local tee mlookup 2",
            meta,
            |meta| curr!(meta, opcode_bit) * enable(meta),
            |meta| curr!(meta, common.eid),
            |_meta| constant_from!(2u64),
            |meta| curr!(meta, common.sp) + curr!(meta, offset),
            |meta| curr!(meta, vtype),
            |meta| curr!(meta, value.value),
        );

        Box::new(LocalTeeConfig {
            offset,
            value,
            vtype,
            enable: opcode_bit,
            _mark: PhantomData,
        })
    }
}

impl<F: FieldExt> EventTableOpcodeConfig<F> for LocalTeeConfig<F> {
    fn opcode(&self, meta: &mut VirtualCells<'_, F>) -> Expression<F> {
        (constant!(bn_to_field(
            &(BigUint::from(OpcodeClass::LocalTee as u64) << OPCODE_CLASS_SHIFT)
        )) + curr!(meta, self.vtype)
            * constant!(bn_to_field(&(BigUint::from(1u64) << OPCODE_ARG0_SHIFT)))
            + curr!(meta, self.offset))
            * curr!(meta, self.enable)
    }

    fn sp_diff(&self, _meta: &mut VirtualCells<'_, F>) -> Expression<F> {
        constant!(F::zero())
    }

    fn opcode_class(&self) -> OpcodeClass {
        OpcodeClass::LocalTee
    }

    fn assign(&self, ctx: &mut Context<'_, F>, entry: &EventTableEntry) -> Result<(), Error> {
        match entry.step_info {
            specs::step::StepInfo::TeeLocal {
                vtype,
                depth,
                value,
            } => {
                ctx.region.assign_advice(
                    || "op_local_tee offset",
                    self.offset,
                    ctx.offset,
                    || Ok(F::from(depth as u64)),
                )?;

                ctx.region.assign_advice(
                    || "op_local_tee vtype",
                    self.vtype,
                    ctx.offset,
                    || Ok(F::from(vtype as u64)),
                )?;

                self.value.assign(ctx, value)?;
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
    use specs::types::Value;
    use wasmi::{ImportsBuilder, NopExternals};

    #[test]
    fn test_local_tee() {
        let textual_repr = r#"
                (module
                    (func (export "test") (param $0 i32)
                      (i32.const 1)
                      (local.tee $0)
                      (drop)
                    )
                   )
                "#;

        let compiler = WasmInterpreter::new();
        let compiled_module = compiler
            .compile(textual_repr, &ImportsBuilder::default())
            .unwrap();
        let execution_log = compiler
            .run(
                &mut NopExternals,
                &compiled_module,
                "test",
                vec![Value::I32(0)],
            )
            .unwrap();

        run_test_circuit::<Fp>(compiled_module.tables, execution_log.tables).unwrap()
    }
}
