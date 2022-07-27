use crate::{
    circuits::{
        etable::{EventTableCommonConfig, EventTableOpcodeConfig, EventTableOpcodeConfigBuilder},
        itable::InstructionTableConfig,
        jtable::JumpTableConfig,
        mtable::MemoryTableConfig,
        rtable::RangeTableConfig,
        utils::{bn_to_field, Context},
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
    itable::{OpcodeClass, OPCODE_CLASS_SHIFT},
    mtable::VarType,
};
use std::marker::PhantomData;

pub struct CallHostTimeConfig<F: FieldExt> {
    ret_val: Column<Advice>,
    enable: Column<Advice>,
    _mark: PhantomData<F>,
}

pub struct CallHostTimeConfigBuilder {}

impl<F: FieldExt> EventTableOpcodeConfigBuilder<F> for CallHostTimeConfigBuilder {
    fn configure(
        meta: &mut ConstraintSystem<F>,
        common: &EventTableCommonConfig,
        opcode_bit: Column<Advice>,
        cols: &mut impl Iterator<Item = Column<Advice>>,
        _rtable: &RangeTableConfig<F>,
        _itable: &InstructionTableConfig<F>,
        mtable: &MemoryTableConfig<F>,
        _jtable: &JumpTableConfig<F>,
        enable: impl Fn(&mut VirtualCells<'_, F>) -> Expression<F>,
    ) -> Box<dyn EventTableOpcodeConfig<F>> {
        let ret_val = cols.next().unwrap();

        mtable.configure_stack_write_in_table(
            "op_host_time ret",
            meta,
            |meta| curr!(meta, opcode_bit) * enable(meta),
            |meta| curr!(meta, common.eid),
            |_meta| constant_from!(1),
            |meta| curr!(meta, common.sp),
            |_meta| constant_from!(VarType::U64),
            |meta| curr!(meta, ret_val),
        );

        Box::new(CallHostTimeConfig {
            ret_val,
            enable: opcode_bit,
            _mark: PhantomData,
        })
    }
}

impl<F: FieldExt> EventTableOpcodeConfig<F> for CallHostTimeConfig<F> {
    fn opcode(&self, meta: &mut VirtualCells<'_, F>) -> Expression<F> {
        constant!(bn_to_field(
            &(BigUint::from(OpcodeClass::CallHostTime as u64) << OPCODE_CLASS_SHIFT)
        )) * curr!(meta, self.enable)
    }

    fn sp_diff(&self, meta: &mut VirtualCells<'_, F>) -> Expression<F> {
        constant!(-F::one()) * curr!(meta, self.enable)
    }

    fn assign(&self, ctx: &mut Context<'_, F>, entry: &EventTableEntry) -> Result<(), Error> {
        match entry.step_info {
            specs::step::StepInfo::CallHostTime { ret_val } => {
                ctx.region.assign_advice(
                    || "func_index",
                    self.ret_val,
                    ctx.offset,
                    || Ok(F::from(ret_val.unwrap())),
                )?;
            }
            _ => unreachable!(),
        }

        Ok(())
    }

    fn opcode_class(&self) -> OpcodeClass {
        OpcodeClass::CallHostTime
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        runtime::{WasmInterpreter, WasmRuntime},
        test::test_circuit_builder::run_test_circuit,
    };
    use halo2_proofs::pairing::bn256::Fr as Fp;
    use specs::host_function::TIME_FUNC_INDEX;
    use wasmi::{
        Error, Externals, FuncInstance, ImportsBuilder, ModuleImportResolver, NopExternals,
        RuntimeArgs, RuntimeValue, Signature, Trap, ValueType,
    };

    struct TestHost {}

    impl TestHost {
        fn new() -> TestHost {
            TestHost {}
        }
    }

    impl Externals for TestHost {
        fn invoke_index(
            &mut self,
            index: usize,
            _args: RuntimeArgs,
        ) -> Result<Option<RuntimeValue>, Trap> {
            match index {
                TIME_FUNC_INDEX => {
                    // let tt = SystemTime::now();
                    // let t: i32 = Instant::now();
                    Ok(Some(0.into()))
                }
                _ => panic!("env doesn't provide function at index {}", index),
            }
        }
    }

    impl TestHost {
        fn check_signature(&self, index: usize, signature: &Signature) -> bool {
            let (params, ret_ty): (&[ValueType], Option<ValueType>) = match index {
                TIME_FUNC_INDEX => (&[], Some(ValueType::I32)),
                _ => return false,
            };

            signature.params() == params && signature.return_type() == ret_ty
        }
    }

    impl ModuleImportResolver for TestHost {
        fn resolve_func(
            &self,
            field_name: &str,
            signature: &wasmi::Signature,
        ) -> Result<wasmi::FuncRef, wasmi::Error> {
            let index = match field_name {
                "time" => TIME_FUNC_INDEX,
                _ => {
                    return Err(Error::Instantiation(format!(
                        "Export {} not found",
                        field_name
                    )));
                }
            };

            if !self.check_signature(index, signature) {
                return Err(Error::Instantiation(format!(
                    "Export `{}` doesnt match expected type {:?}",
                    field_name, signature
                )));
            }

            Ok(FuncInstance::alloc_host(signature.clone(), index))
        }
    }

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
        let compiled_module = compiler
            .compile(textual_repr, &ImportsBuilder::default())
            .unwrap();
        let execution_log = compiler
            .run(&mut NopExternals, &compiled_module, "test", vec![])
            .unwrap();
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
        let compiled_module = compiler
            .compile(textual_repr, &ImportsBuilder::default())
            .unwrap();
        let execution_log = compiler
            .run(&mut NopExternals, &compiled_module, "test", vec![])
            .unwrap();
        run_test_circuit::<Fp>(compiled_module.tables, execution_log.tables).unwrap()
    }

    #[test]
    fn test_host_function() {
        let textual_repr = r#"
            (module
                (import "env" "time" (func $time (result i32)))
                (func (export "test")
                    (call $time)
                    drop
                )
            )
        "#;

        let compiler = WasmInterpreter::new();
        let mut env = TestHost::new();
        let imports = ImportsBuilder::new().with_resolver("env", &env);
        let compiled_module = compiler.compile(textual_repr, &imports).unwrap();
        let execution_log = compiler
            .run(&mut env, &compiled_module, "test", vec![])
            .unwrap();
        run_test_circuit::<Fp>(compiled_module.tables, execution_log.tables).unwrap()
    }
}
