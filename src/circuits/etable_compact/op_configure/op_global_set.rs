use super::*;
use crate::{
    circuits::{
        imtable::IMTableEncode, mtable_compact::encode::MemoryTableLookupEncode, utils::Context,
    },
    constant,
};
use halo2_proofs::{
    arithmetic::FieldExt,
    plonk::{Error, Expression, VirtualCells},
};
use specs::{encode::opcode::encode_global_set, mtable::LocationType, step::StepInfo};
use specs::{etable::EventTableEntry, itable::OpcodeClass};

pub struct GlobalSetConfig {
    origin_moid: CommonRangeCell,
    origin_idx: CommonRangeCell,
    local: BitCell,
    imtable_lookup: IMTableLookupCell,
    idx: CommonRangeCell,
    vtype: CommonRangeCell,
    value: U64Cell,
    lookup_stack_read: MTableLookupCell,
    lookup_global_set: MTableLookupCell,
}

pub struct GlobalSetConfigBuilder {}

impl<F: FieldExt> EventTableOpcodeConfigBuilder<F> for GlobalSetConfigBuilder {
    fn configure(
        common: &mut EventTableCellAllocator<F>,
        constraint_builder: &mut ConstraintBuilder<F>,
    ) -> Box<dyn EventTableOpcodeConfig<F>> {
        let origin_moid = common.alloc_common_range_value();
        let origin_idx = common.alloc_common_range_value();
        let moid = common.moid_cell();
        let idx = common.alloc_common_range_value();
        let local = common.alloc_bit_value();
        let imtable_lookup = common.alloc_imtable_lookup();

        let vtype = common.alloc_common_range_value();
        let value = common.alloc_u64();

        let lookup_stack_read = common.alloc_mtable_lookup();
        let lookup_global_set = common.alloc_mtable_lookup();

        constraint_builder.push(
            "op_global_get imported",
            Box::new(move |meta| {
                vec![local.expr(meta) * (origin_moid.expr(meta) - moid.expr(meta))]
            }),
        );

        Box::new(GlobalSetConfig {
            origin_moid,
            origin_idx,
            local,
            imtable_lookup,
            idx,
            vtype,
            value,
            lookup_stack_read,
            lookup_global_set,
        })
    }
}

impl<F: FieldExt> EventTableOpcodeConfig<F> for GlobalSetConfig {
    fn opcode(&self, meta: &mut VirtualCells<'_, F>) -> Expression<F> {
        encode_global_set(self.idx.expr(meta))
    }

    fn assign(
        &self,
        ctx: &mut Context<'_, F>,
        step_info: &StepStatus,
        entry: &EventTableEntry,
    ) -> Result<(), Error> {
        match &entry.step_info {
            StepInfo::SetGlobal {
                idx,
                origin_module,
                origin_idx,
                vtype,
                value,
                ..
            } => {
                self.idx.assign(ctx, *idx as u16)?;
                self.origin_idx.assign(ctx, *origin_idx as u16)?;
                self.origin_moid.assign(ctx, *origin_module as u16)?;
                self.vtype.assign(ctx, *vtype as u16)?;
                self.value.assign(ctx, *value)?;
                self.local
                    .assign(ctx, *origin_module == step_info.current.moid)?;

                self.lookup_stack_read.assign(
                    ctx,
                    &MemoryTableLookupEncode::encode_stack_read(
                        BigUint::from(step_info.current.eid),
                        BigUint::from(1 as u64),
                        BigUint::from(step_info.current.sp + 1),
                        BigUint::from(*vtype as u16),
                        BigUint::from(*value),
                    ),
                )?;

                self.lookup_global_set.assign(
                    ctx,
                    &MemoryTableLookupEncode::encode_global_set(
                        BigUint::from(step_info.current.eid),
                        BigUint::from(2 as u64),
                        BigUint::from(*origin_module as u64),
                        BigUint::from(*origin_idx as u64),
                        BigUint::from(*vtype as u64),
                        BigUint::from(*value),
                    ),
                )?;

                if *origin_module != step_info.current.moid {
                    self.imtable_lookup.assign(
                        ctx,
                        &IMTableEncode::encode_for_import(
                            BigUint::from(LocationType::Global as u64),
                            BigUint::from(*origin_module),
                            BigUint::from(*origin_idx),
                            BigUint::from(step_info.current.moid),
                            BigUint::from(*idx),
                        ),
                    )?;
                }

                Ok(())
            }

            _ => unreachable!(),
        }
    }

    fn opcode_class(&self) -> OpcodeClass {
        // Delete opcode_class
        OpcodeClass::GlobalSet
    }

    fn sp_diff(&self, _meta: &mut VirtualCells<'_, F>) -> Option<Expression<F>> {
        Some(constant!(F::one()))
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
                common_config.sp(meta) + constant_from!(1),
                self.vtype.expr(meta),
                self.value.expr(meta),
            )),
            MLookupItem::Second => Some(MemoryTableLookupEncode::encode_global_set(
                common_config.eid(meta),
                constant_from!(2),
                self.origin_moid.expr(meta),
                self.origin_idx.expr(meta),
                self.vtype.expr(meta),
                self.value.expr(meta),
            )),
            _ => None,
        }
    }

    fn imtable_lookup(
        &self,
        meta: &mut VirtualCells<'_, F>,
        common_config: &EventTableCommonConfig<F>,
    ) -> Option<Expression<F>> {
        Some(
            (constant_from!(1) - self.local.expr(meta))
                * IMTableEncode::encode_for_import(
                    constant_from!(LocationType::Global),
                    self.origin_moid.expr(meta),
                    self.origin_idx.expr(meta),
                    common_config.moid(meta),
                    self.idx.expr(meta),
                ),
        )
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use halo2_proofs::pairing::bn256::Fr as Fp;
    use wasmi::ImportsBuilder;

    use crate::{
        runtime::{host::HostEnv, WasmInterpreter, WasmRuntime},
        test::{run_test_circuit, test_circuit_noexternal},
    };

    #[test]
    fn test_global_set() {
        let textual_repr = r#"
                (module
                    (global $global_i32 (mut i32) (i32.const 10))

                    (func (export "test")
                        (i32.const 0)
                        (global.set $global_i32)
                    )
                )
                "#;

        test_circuit_noexternal(textual_repr).unwrap()
    }

    #[test]
    fn test_global_set_import_other_instance() {
        let compiler = WasmInterpreter::new(HashMap::default());
        let mut env = HostEnv::new();

        let instance_export = {
            let mod_export = r#"
                (module
                  (global (export "global-i32") (mut i32) (i32.const 100))
                )
              "#;

            let mod_export = wabt::wat2wasm(mod_export).expect("failed to parse wat");
            let imports = &ImportsBuilder::default();
            compiler.compile(&mod_export, imports).unwrap()
        };

        env.register_global_ref(
            "global-i32",
            instance_export
                .export_by_name("global-i32")
                .unwrap()
                .as_global()
                .unwrap()
                .clone(),
        )
        .unwrap();

        let instance_import = {
            let mod_import = r#"
              (module
                (import "env" "global-i32" (global (mut i32)))

                (func (export "test")
                  (i32.const 0)
                  (global.set 0)
                )
              )
            "#;

            let mod_import = wabt::wat2wasm(&mod_import).expect("failed to parse wat");
            let imports = ImportsBuilder::new().with_resolver("env", &env);
            compiler.compile(&mod_import, &imports).unwrap()
        };

        let _ = compiler
            .run(&mut env, &instance_import, "test", vec![], vec![])
            .unwrap();

        run_test_circuit::<Fp>(
            compiler.compile_table(),
            compiler.execution_tables(),
            vec![],
        )
        .unwrap()
    }
}
