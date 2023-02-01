use super::*;
use crate::{
    circuits::{mtable_compact::encode::MemoryTableLookupEncode, utils::Context},
    constant,
};

use halo2_proofs::{
    arithmetic::FieldExt,
    plonk::{Error, Expression, VirtualCells},
};
use specs::{
    encode::opcode::encode_call_host,
    etable::EventTableEntry,
    external_host_call_table::{encode::encode_host_call_entry, ExternalHostCallSignature},
    mtable::VarType,
    step::StepInfo,
};

pub struct ExternalCallHostCircuitConfig {
    op: CommonRangeCell,
    value: U64Cell,
    value_is_ret: BitCell,
    stack_rw_lookup: MTableLookupCell,
    external_host_call_lookup: ExternalHostCallTableLookupCell,
}

pub struct ExternalCallHostCircuitConfigBuilder {}

impl<F: FieldExt> EventTableOpcodeConfigBuilder<F> for ExternalCallHostCircuitConfigBuilder {
    fn configure(
        common: &mut EventTableCellAllocator<F>,
        constraint_builder: &mut ConstraintBuilder<F>,
    ) -> Box<dyn EventTableOpcodeConfig<F>> {
        let index = common.external_host_index_cell();
        let op = common.alloc_common_range_value();
        let value = common.alloc_u64();
        let value_is_ret = common.alloc_bit_value();

        let stack_rw_lookup = common.alloc_mtable_lookup();
        let external_host_call_lookup = common.alloc_external_host_call_table_lookup();

        constraint_builder.push(
            "external host call lookup",
            Box::new(move |meta| {
                vec![
                    external_host_call_lookup.clone().expr(meta)
                        - encode_host_call_entry(
                            index.expr(meta),
                            op.expr(meta),
                            value_is_ret.expr(meta),
                            value.expr(meta),
                        ),
                ]
            }),
        );

        Box::new(ExternalCallHostCircuitConfig {
            op,
            value,
            value_is_ret,
            stack_rw_lookup,
            external_host_call_lookup,
        })
    }
}

impl<F: FieldExt> EventTableOpcodeConfig<F> for ExternalCallHostCircuitConfig {
    fn opcode(&self, meta: &mut VirtualCells<'_, F>) -> Expression<F> {
        encode_call_host(self.op.expr(meta), self.value_is_ret.expr(meta))
    }

    fn assign(
        &self,
        ctx: &mut Context<'_, F>,
        step_info: &StepStatus,
        entry: &EventTableEntry,
    ) -> Result<(), Error> {
        match &entry.step_info {
            StepInfo::ExternalHostCall { op, value, sig } => {
                self.op.assign(ctx, F::from(*op as u64))?;
                self.value.assign(ctx, value.unwrap())?;
                self.value_is_ret.assign(ctx, sig.is_ret())?;
                self.external_host_call_lookup.assign(
                    ctx,
                    &encode_host_call_entry(
                        BigUint::from(step_info.current_external_host_call_index),
                        BigUint::from(*op as u64),
                        BigUint::from(sig.is_ret() as u64),
                        BigUint::from(value.unwrap()),
                    ),
                )?;

                match sig {
                    ExternalHostCallSignature::Argument => {
                        self.stack_rw_lookup.assign(
                            ctx,
                            &MemoryTableLookupEncode::encode_stack_read(
                                BigUint::from(step_info.current.eid),
                                BigUint::from(1 as u64),
                                BigUint::from(step_info.current.sp + 1),
                                BigUint::from(VarType::I64 as u64),
                                BigUint::from(value.unwrap()),
                            ),
                        )?;
                    }
                    ExternalHostCallSignature::Return => {
                        self.stack_rw_lookup.assign(
                            ctx,
                            &MemoryTableLookupEncode::encode_stack_write(
                                BigUint::from(step_info.current.eid),
                                BigUint::from(1 as u64),
                                BigUint::from(step_info.current.sp),
                                BigUint::from(VarType::I64 as u64),
                                BigUint::from(value.unwrap()),
                            ),
                        )?;
                    }
                }

                Ok(())
            }

            _ => unreachable!(),
        }
    }

    fn mops(&self, _meta: &mut VirtualCells<'_, F>) -> Option<Expression<F>> {
        Some(constant_from!(1))
    }

    fn mtable_lookup(
        &self,
        meta: &mut VirtualCells<'_, F>,
        item: MLookupItem,
        common_config: &EventTableCommonConfig<F>,
    ) -> Option<Expression<F>> {
        match item {
            MLookupItem::First => Some(
                (constant_from!(1) - self.value_is_ret.expr(meta))
                    * MemoryTableLookupEncode::encode_stack_read(
                        common_config.eid(meta),
                        constant_from!(1),
                        common_config.sp(meta) + constant_from!(1),
                        constant_from!(VarType::I64 as u64),
                        self.value.expr(meta),
                    )
                    + self.value_is_ret.expr(meta)
                        * MemoryTableLookupEncode::encode_stack_write(
                            common_config.eid(meta),
                            constant_from!(1),
                            common_config.sp(meta),
                            constant_from!(VarType::I64 as u64),
                            self.value.expr(meta),
                        ),
            ),
            _ => None,
        }
    }

    fn sp_diff(&self, meta: &mut VirtualCells<'_, F>) -> Option<Expression<F>> {
        Some(
            self.value_is_ret.expr(meta) * constant!(-F::one())
                + (constant_from!(1) - self.value_is_ret.expr(meta)),
        )
    }

    fn external_host_call_index_increase(
        &self,
        _meta: &mut VirtualCells<'_, F>,
        _common_config: &EventTableCommonConfig<F>,
    ) -> bool {
        true
    }
}

#[cfg(test)]

mod tests {
    use halo2_proofs::pairing::bn256::Fr as Fp;
    use specs::external_host_call_table::ExternalHostCallSignature;
    use std::rc::Rc;
    use wasmi::ImportsBuilder;

    use crate::{
        runtime::{
            host::{host_env::HostEnv, ForeignContext},
            wasmi_interpreter::Execution,
            WasmInterpreter, WasmRuntime,
        },
        test::run_test_circuit,
    };

    #[derive(Default)]
    struct Context {
        acc: u64,
    }
    impl ForeignContext for Context {}

    #[test]
    fn test_call_host() {
        let textual_repr = r#"
        (module
            (type (;0;) (func (result i64)))
            (type (;1;) (func (param i64)))
            (import "env" "foreign_push" (func (;0;) (type 1)))
            (import "env" "foreign_pop" (func (;1;) (type 0)))
            (func (;2;) (type 0) (result i64)
              i64.const 5
              call 0
              i64.const 10
              call 0
              i64.const 3
              call 0
              call 1)
            (memory (;0;) 1)
            (export "memory" (memory 0))
            (export "test" (func 2)))
        "#;

        let wasm = wabt::wat2wasm(&textual_repr).expect("failed to parse wat");

        let compiler = WasmInterpreter::new();
        let mut env = {
            let mut env = HostEnv::new();

            let foreign_playground_plugin = env
                .external_env
                .register_plugin("foreign_playground", Box::new(Context::default()));
            env.external_env.register_function(
                "foreign_push",
                0,
                ExternalHostCallSignature::Argument,
                foreign_playground_plugin.clone(),
                Rc::new(
                    |context: &mut dyn ForeignContext, args: wasmi::RuntimeArgs| {
                        let context = context.downcast_mut::<Context>().unwrap();

                        let value: u64 = args.nth(0);
                        context.acc += value;

                        None
                    },
                ),
            );
            env.external_env.register_function(
                "foreign_pop",
                1,
                ExternalHostCallSignature::Return,
                foreign_playground_plugin,
                Rc::new(
                    |context: &mut dyn ForeignContext, _args: wasmi::RuntimeArgs| {
                        let context = context.downcast_mut::<Context>().unwrap();

                        Some(wasmi::RuntimeValue::I64(context.acc as i64))
                    },
                ),
            );

            env.finalize();

            env
        };

        let imports = ImportsBuilder::new().with_resolver("env", &env);
        let compiled_module = compiler
            .compile(&wasm, &imports, &env.function_description_table())
            .unwrap();
        let execution_result = compiled_module.run(&mut env, "test").unwrap();
        run_test_circuit::<Fp>(execution_result.tables, vec![]).unwrap()
    }
}
