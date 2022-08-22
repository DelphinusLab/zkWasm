use super::*;
use crate::{
    circuits::{
        mtable_compact::encode::MemoryTableLookupEncode,
        utils::{bn_to_field, Context},
    },
    constant,
};
use halo2_proofs::{
    arithmetic::FieldExt,
    plonk::{Error, Expression, VirtualCells},
};
use specs::step::StepInfo;
use specs::{
    etable::EventTableEntry,
    itable::{OpcodeClass, OPCODE_ARG0_SHIFT, OPCODE_CLASS_SHIFT},
};

pub struct LocalGetConfig {
    offset: CommonRangeCell,
    vtype: CommonRangeCell,
    value: U64Cell,
    lookup_stack_read: MTableLookupCell,
    lookup_stack_write: MTableLookupCell,
}

pub struct LocalGetConfigBuilder {}

impl<F: FieldExt> EventTableOpcodeConfigBuilder<F> for LocalGetConfigBuilder {
    fn configure(
        common: &mut EventTableCellAllocator<F>,
        _constraint_builder: &mut ConstraintBuilder<F>,
    ) -> Box<dyn EventTableOpcodeConfig<F>> {
        let offset = common.alloc_common_range_value();
        let vtype = common.alloc_common_range_value();
        let value = common.alloc_u64();
        let lookup_stack_read = common.alloc_mtable_lookup();
        let lookup_stack_write = common.alloc_mtable_lookup();

        Box::new(LocalGetConfig {
            offset,
            vtype,
            value,
            lookup_stack_read,
            lookup_stack_write,
        })
    }
}

impl<F: FieldExt> EventTableOpcodeConfig<F> for LocalGetConfig {
    fn opcode(&self, meta: &mut VirtualCells<'_, F>) -> Expression<F> {
        constant!(bn_to_field(
            &(BigUint::from(OpcodeClass::LocalGet as u64) << OPCODE_CLASS_SHIFT)
        )) + self.vtype.expr(meta)
            * constant!(bn_to_field(&(BigUint::from(1u64) << OPCODE_ARG0_SHIFT)))
            + self.offset.expr(meta)
    }

    fn assign(
        &self,
        ctx: &mut Context<'_, F>,
        step_info: &StepStatus,
        entry: &EventTableEntry,
    ) -> Result<(), Error> {
        match &entry.step_info {
            StepInfo::GetLocal {
                vtype,
                depth,
                value,
            } => {
                self.vtype.assign(ctx, *vtype as u16)?;
                self.value.assign(ctx, *value)?;
                self.offset.assign(ctx, *depth as u16)?;

                self.lookup_stack_read.assign(
                    ctx,
                    &MemoryTableLookupEncode::encode_stack_read(
                        BigUint::from(step_info.current.eid),
                        BigUint::from(1 as u64),
                        BigUint::from(step_info.current.sp + *depth as u64),
                        BigUint::from(*vtype as u16),
                        BigUint::from(*value),
                    ),
                )?;

                self.lookup_stack_write.assign(
                    ctx,
                    &MemoryTableLookupEncode::encode_stack_write(
                        BigUint::from(step_info.current.eid),
                        BigUint::from(2 as u64),
                        BigUint::from(step_info.current.sp),
                        BigUint::from(*vtype as u16),
                        BigUint::from(*value),
                    ),
                )?;

                Ok(())
            }

            _ => unreachable!(),
        }
    }

    fn opcode_class(&self) -> OpcodeClass {
        OpcodeClass::LocalGet
    }

    fn sp_diff(&self, _meta: &mut VirtualCells<'_, F>) -> Option<Expression<F>> {
        Some(constant!(-F::one()))
    }

    fn mops(&self, _meta: &mut VirtualCells<'_, F>) -> Option<Expression<F>> {
        Some(constant_from!(2))
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
                common_config.sp(meta) + self.offset.expr(meta),
                self.vtype.expr(meta),
                self.value.expr(meta),
            )),
            MLookupItem::Second => Some(MemoryTableLookupEncode::encode_stack_write(
                common_config.eid(meta),
                constant_from!(2),
                common_config.sp(meta),
                self.vtype.expr(meta),
                self.value.expr(meta),
            )),
            MLookupItem::Third => None,
            _ => None,
        }
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
    fn test_local_get() {
        let textual_repr = r#"
                (module
                    (func (export "test") (param $0 i32)
                      (local.get $0)
                      (drop)
                    )
                   )
                "#;

        let wasm = wabt::wat2wasm(textual_repr).unwrap();

        let compiler = WasmInterpreter::new();
        let compiled_module = compiler.compile(&wasm, &ImportsBuilder::default()).unwrap();
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
