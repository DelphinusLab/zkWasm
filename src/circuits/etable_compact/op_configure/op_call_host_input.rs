use super::*;
use crate::{
    circuits::{
        intable::InputTableEncode,
        mtable_compact::encode::MemoryTableLookupEncode,
        utils::{bn_to_field, Context},
    },
    constant,
    foreign::ForeignCallInfo,
};

use halo2_proofs::{
    arithmetic::FieldExt,
    plonk::{Error, Expression, VirtualCells},
};
use specs::{
    etable::EventTableEntry,
    host_function::HostPlugin,
    itable::{OpcodeClass, OPCODE_CLASS_SHIFT},
};
use specs::{mtable::VarType, step::StepInfo};

pub struct CallHostWasmInputConfig {
    public: BitCell,
    value: U64Cell,

    lookup_read_stack: MTableLookupCell,
    lookup_write_stack: MTableLookupCell,
}

pub struct CallHostWasmInputConfigBuilder {}

impl<F: FieldExt> EventTableForeignCallConfigBuilder<F> for CallHostWasmInputConfigBuilder {
    fn configure(
        common: &mut EventTableCellAllocator<F>,
        constraint_builder: &mut ConstraintBuilder<F>,
        _info: &impl ForeignCallInfo,
    ) -> Box<dyn EventTableOpcodeConfig<F>> {
        let public = common.alloc_bit_value();
        let value = common.alloc_u64();

        let lookup_read_stack = common.alloc_mtable_lookup();
        let lookup_write_stack = common.alloc_mtable_lookup();

        let input_index = common.input_index_cell();

        constraint_builder.lookup(
            INPUT_TABLE_KEY,
            "lookup input table",
            Box::new(move |meta| {
                InputTableEncode::encode_for_lookup(input_index.expr(meta), value.expr(meta))
            }),
        );

        Box::new(CallHostWasmInputConfig {
            public,
            value,
            lookup_read_stack,
            lookup_write_stack,
        })
    }
}

impl<F: FieldExt> EventTableOpcodeConfig<F> for CallHostWasmInputConfig {
    fn opcode(&self, _meta: &mut VirtualCells<'_, F>) -> Expression<F> {
        constant!(bn_to_field(
            &(BigUint::from(OpcodeClass::ForeignPluginStart as u64 + HostPlugin::HostInput as u64)
                << OPCODE_CLASS_SHIFT)
        ))
    }

    fn assign(
        &self,
        ctx: &mut Context<'_, F>,
        step_info: &StepStatus,
        entry: &EventTableEntry,
    ) -> Result<(), Error> {
        match &entry.step_info {
            StepInfo::CallHost {
                plugin,
                host_function_idx,
                args,
                ret_val,
                signature,
                ..
            } => {
                assert_eq!(*plugin, HostPlugin::HostInput);
                assert_eq!(args.len(), 1);

                self.public.assign(ctx, (*args.get(0).unwrap()) == 1)?;
                self.value.assign(ctx, ret_val.unwrap())?;

                let arg_type: VarType = (*signature.params.get(0).unwrap()).into();
                let ret_type: VarType = signature.return_type.unwrap().into();

                self.lookup_read_stack.assign(
                    ctx,
                    &MemoryTableLookupEncode::encode_stack_read(
                        BigUint::from(step_info.current.eid),
                        BigUint::from(1 as u64),
                        BigUint::from(step_info.current.sp + 1),
                        BigUint::from(arg_type as u16),
                        BigUint::from(*args.get(0).unwrap()),
                    ),
                )?;

                self.lookup_write_stack.assign(
                    ctx,
                    &MemoryTableLookupEncode::encode_stack_write(
                        BigUint::from(step_info.current.eid),
                        BigUint::from(2 as u64),
                        BigUint::from(step_info.current.sp + 1),
                        BigUint::from(ret_type as u16),
                        BigUint::from(ret_val.unwrap()),
                    ),
                )?;

                Ok(())
            }

            _ => unreachable!(),
        }
    }

    fn opcode_class(&self) -> OpcodeClass {
        unreachable!()
    }

    fn mops(&self, _meta: &mut VirtualCells<'_, F>) -> Option<Expression<F>> {
        Some(constant_from!(2))
    }

    fn assigned_extra_mops(
        &self,
        _ctx: &mut Context<'_, F>,
        _step: &StepStatus,
        _entry: &EventTableEntry,
    ) -> u64 {
        2
    }

    fn mtable_lookup(
        &self,
        meta: &mut VirtualCells<'_, F>,
        item: MLookupItem,
        common_config: &EventTableCommonConfig<F>,
    ) -> Option<Expression<F>> {
        match item {
            MLookupItem::First => Some(MemoryTableLookupEncode::encode_stack_read(
                common_config.eid(meta),
                constant_from!(1),
                common_config.sp(meta) + constant_from!(1),
                constant_from!(VarType::I32),
                self.public.expr(meta),
            )),
            MLookupItem::Second => Some(MemoryTableLookupEncode::encode_stack_write(
                common_config.eid(meta),
                constant_from!(2),
                common_config.sp(meta) + constant_from!(1),
                constant_from!(VarType::I64),
                self.value.expr(meta),
            )),
            _ => None,
        }
    }

    fn input_index_increase(
        &self,
        meta: &mut VirtualCells<'_, F>,
        _common_config: &EventTableCommonConfig<F>,
    ) -> Option<Expression<F>> {
        Some(self.public.expr(meta))
    }

    fn is_host_public_input(&self, _step: &StepStatus, entry: &EventTableEntry) -> bool {
        match &entry.step_info {
            StepInfo::CallHost {
                plugin,
                args,
                ..
            } => {
                assert_eq!(*plugin, HostPlugin::HostInput);
                assert_eq!(args.len(), 1);

                args[0] == 1
            }
            _ => unreachable!(),
        }
    }
}

#[cfg(test)]

mod tests {
    use halo2_proofs::pairing::bn256::Fr as Fp;
    use wasmi::ImportsBuilder;

    use crate::{
        runtime::{host::HostEnv, WasmInterpreter, WasmRuntime},
        test::run_test_circuit,
    };

    #[test]
    fn test_wasm_input() {
        let textual_repr = r#"
        (module
            (import "env" "wasm_input" (func $wasm_input (param i32) (result i64)))
            (export "main" (func $main))
            (func $main (; 1 ;) (result i32)
             (drop
              (call $wasm_input
               (i32.const 0)
              )
             )
             (i32.const 0)
            )
           )
        "#;

        let wasm = wabt::wat2wasm(textual_repr).unwrap();
        let mut env = HostEnv::new();

        let compiler = WasmInterpreter::new();
        let imports = ImportsBuilder::new().with_resolver("env", &env);
        let compiled_module = compiler
            .compile(&wasm, &imports, &env.function_plugin_lookup)
            .unwrap();
        let execution_log = compiler
            .run(&mut env, &compiled_module, "main", vec![])
            .unwrap();
        run_test_circuit::<Fp>(compiled_module.tables, execution_log.tables).unwrap()
    }
}
