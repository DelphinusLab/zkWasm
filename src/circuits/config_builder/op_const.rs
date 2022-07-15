use crate::{
    circuits::{
        etable::{EventTableCommonConfig, EventTableOpcodeConfig, EventTableOpcodeConfigBuilder},
        itable::InstructionTableConfig,
        jtable::JumpTableConfig,
        mtable::MemoryTableConfig,
        rtable::RangeTableConfig,
        utils::{bn_to_field, tvalue::TValueConfig, Context},
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

pub struct ConstConfig<F: FieldExt> {
    tvalue: TValueConfig<F>,
    enable: Column<Advice>,
}

pub struct ConstConfigBuilder {}

impl<F: FieldExt> EventTableOpcodeConfigBuilder<F> for ConstConfigBuilder {
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
        let tvalue = TValueConfig::configure(meta, cols, rtable, |meta| {
            curr!(meta, opcode_bit) * enable(meta)
        });

        mtable.configure_stack_write_in_table(
            "const mlookup",
            meta,
            |meta| curr!(meta, opcode_bit) * enable(meta),
            |meta| curr!(meta, common.eid),
            |_meta| constant_from!(1),
            |meta| curr!(meta, common.sp),
            |meta| curr!(meta, tvalue.vtype),
            |meta| curr!(meta, tvalue.value.value),
        );

        Box::new(ConstConfig {
            enable: opcode_bit,
            tvalue,
        })
    }
}

impl<F: FieldExt> EventTableOpcodeConfig<F> for ConstConfig<F> {
    fn opcode(&self, meta: &mut VirtualCells<'_, F>) -> Expression<F> {
        // FIXME
        (constant!(bn_to_field(
            &(BigUint::from(OpcodeClass::Const as u64) << OPCODE_CLASS_SHIFT)
        )) + curr!(meta, self.tvalue.vtype)
            * constant!(bn_to_field(&(BigUint::from(1u64) << OPCODE_ARG0_SHIFT)))
            + curr!(meta, self.tvalue.value.value))
            * curr!(meta, self.enable)
    }

    fn sp_diff(&self, meta: &mut VirtualCells<'_, F>) -> Expression<F> {
        constant!(-F::one()) * curr!(meta, self.enable)
    }

    fn opcode_class(&self) -> OpcodeClass {
        OpcodeClass::Const
    }

    fn assign(&self, ctx: &mut Context<'_, F>, entry: &EventTableEntry) -> Result<(), Error> {
        match entry.step_info {
            specs::step::StepInfo::I32Const { value } => {
                self.tvalue.assign(ctx, VarType::I32, value as u32 as u64)?;
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
    fn test_ok() {
        let textual_repr = r#"
                (module
                    (func (export "test")
                      (i32.const 0)
                      (drop)
                    )
                   )
                "#;

        let compiler = WasmInterpreter::new();
        let compiled_module = compiler.compile(textual_repr).unwrap();
        let execution_log = compiler.run(&compiled_module, "test", vec![]).unwrap();
println!("{:?}", execution_log.tables.etable);
println!("{:?}", execution_log.tables.mtable);
        run_test_circuit::<Fp>(compiled_module.tables, execution_log.tables).unwrap()
    }
}
