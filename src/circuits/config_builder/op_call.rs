use std::marker::PhantomData;

use crate::{
    circuits::{
        etable::{EventTableCommonConfig, EventTableOpcodeConfig, EventTableOpcodeConfigBuilder},
        itable::InstructionTableConfig,
        jtable::JumpTableConfig,
        mtable::MemoryTableConfig,
        rtable::RangeTableConfig,
        utils::{bn_to_field, Context},
    },
    constant, constant_from, curr, fixed_curr, next,
};
use halo2_proofs::{
    arithmetic::FieldExt,
    plonk::{Advice, Column, ConstraintSystem, Error, Expression, Fixed, VirtualCells},
};
use num_bigint::BigUint;
use specs::{
    etable::EventTableEntry,
    itable::{OpcodeClass, OPCODE_ARG0_SHIFT, OPCODE_CLASS_SHIFT},
};

pub struct CallConfig<F: FieldExt> {
    func_index: Column<Advice>,
    enable: Column<Advice>,
    _mark: PhantomData<F>,
}

pub struct CallConfigBuilder {}

impl<F: FieldExt> EventTableOpcodeConfigBuilder<F> for CallConfigBuilder {
    fn configure(
        meta: &mut ConstraintSystem<F>,
        common: &EventTableCommonConfig,
        opcode_bit: Column<Advice>,
        cols: &mut impl Iterator<Item = Column<Advice>>,
        rtable: &RangeTableConfig<F>,
        _itable: &InstructionTableConfig<F>,
        _mtable: &MemoryTableConfig<F>,
        jtable: &JumpTableConfig<F>,
        enable: impl Fn(&mut VirtualCells<'_, F>) -> Expression<F>,
    ) -> Box<dyn EventTableOpcodeConfig<F>> {
        let func_index = cols.next().unwrap();

        rtable.configure_in_u16_range(meta, "op br pc func_index limit", |meta| {
            curr!(meta, func_index) * curr!(meta, opcode_bit) * enable(meta)
        });

        meta.create_gate("br pc jump", |meta| {
            vec![
                next!(meta, common.iid) * curr!(meta, opcode_bit) * enable(meta),
                (next!(meta, common.fid) - curr!(meta, func_index))
                    * curr!(meta, opcode_bit)
                    * enable(meta),
            ]
        });

        jtable.configure_in_table(
            meta,
            |meta| curr!(meta, opcode_bit) * enable(meta),
            |meta| curr!(meta, common.eid),
            |meta| curr!(meta, common.last_jump_eid),
            |meta| curr!(meta, common.moid),
            |meta| curr!(meta, common.fid),
            |meta| curr!(meta, common.iid) + constant_from!(1),
        );

        Box::new(CallConfig {
            func_index,
            enable: opcode_bit,
            _mark: PhantomData,
        })
    }
}

impl<F: FieldExt> EventTableOpcodeConfig<F> for CallConfig<F> {
    fn opcode(&self, meta: &mut VirtualCells<'_, F>) -> Expression<F> {
        (constant!(bn_to_field(
            &(BigUint::from(OpcodeClass::Call as u64) << OPCODE_CLASS_SHIFT)
        )) + curr!(meta, self.func_index)
            * constant!(bn_to_field(&(BigUint::from(1u64) << OPCODE_ARG0_SHIFT))))
            * curr!(meta, self.enable)
    }

    fn sp_diff(&self, meta: &mut VirtualCells<'_, F>) -> Expression<F> {
        constant!(F::zero())
    }

    fn assign(&self, ctx: &mut Context<'_, F>, entry: &EventTableEntry) -> Result<(), Error> {
        match entry.step_info {
            specs::step::StepInfo::Call { index } => {
                ctx.region.assign_advice(
                    || "func_index",
                    self.func_index,
                    ctx.offset,
                    || Ok(F::from(index as u64)),
                )?;
            }
            _ => unreachable!(),
        }

        Ok(())
    }

    fn opcode_class(&self) -> OpcodeClass {
        OpcodeClass::Call
    }

    fn handle_iid(&self) -> bool {
        true
    }

    fn handle_fid(&self) -> bool {
        true
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
    fn test_call_ok() {
        let textual_repr = r#"
                (module
                    (func $dummy)

                    (func (export "test")
                      (block
                        (call $dummy)
                      )
                    )
                   )
                "#;

        let compiler = WasmInterpreter::new();
        let compiled_module = compiler.compile(textual_repr).unwrap();
        let execution_log = compiler.run(&compiled_module, "test", vec![]).unwrap();
        run_test_circuit::<Fp>(compiled_module.tables, execution_log.tables).unwrap()
    }

    #[test]
    fn test_call_with_arg_ok() {
        let textual_repr = r#"
                (module
                    (func $dummy (param i32))

                    (func (export "test")
                      (block
                        (i32.const 0)
                        (call $dummy)
                      )
                    )
                   )
                "#;

        let compiler = WasmInterpreter::new();
        let compiled_module = compiler.compile(textual_repr).unwrap();
        let execution_log = compiler.run(&compiled_module, "test", vec![]).unwrap();
        run_test_circuit::<Fp>(compiled_module.tables, execution_log.tables).unwrap()
    }
}
