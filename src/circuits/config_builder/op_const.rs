use crate::{
    circuits::{
        etable::{EventTableCommonConfig, EventTableOpcodeConfig, EventTableOpcodeConfigBuilder},
        itable::InstTableConfig,
        jtable::JumpTableConfig,
        mtable::MemoryTableConfig,
        utils::bn_to_field,
    },
    constant, constant_from, curr,
    spec::itable::{OpcodeClass, OPCODE_CLASS_SHIFT, OPCODE_CONST_VTYPE_SHIFT},
};
use halo2_proofs::{
    arithmetic::FieldExt,
    plonk::{Advice, Column, ConstraintSystem, Expression, VirtualCells},
};
use num_bigint::BigUint;
use std::marker::PhantomData;

pub struct ConstConfig<F: FieldExt> {
    vtype: Column<Advice>,
    value: Column<Advice>,
    enable: Column<Advice>,
    _mark: PhantomData<F>,
}

pub struct ConstConfigBuilder {}

impl<F: FieldExt> EventTableOpcodeConfigBuilder<F> for ConstConfigBuilder {
    fn configure(
        meta: &mut ConstraintSystem<F>,
        common: &EventTableCommonConfig,
        opcode_bit: Column<Advice>,
        cols: &mut impl Iterator<Item = Column<Advice>>,
        _itable: &InstTableConfig<F>,
        mtable: &MemoryTableConfig<F>,
        _jtable: &JumpTableConfig<F>,
    ) -> Box<dyn EventTableOpcodeConfig<F>> {
        let value = cols.next().unwrap();
        let vtype = cols.next().unwrap();

        mtable.configure_stack_write_in_table(
            "const mlookup",
            "const mlookup rev",
            meta,
            |meta| curr!(meta, opcode_bit),
            |meta| curr!(meta, common.eid),
            |_meta| constant_from!(1),
            |meta| curr!(meta, common.sp),
            |meta| curr!(meta, vtype),
            |meta| curr!(meta, value),
        );

        Box::new(ConstConfig {
            enable: opcode_bit,
            value,
            vtype,
            _mark: PhantomData,
        })
    }
}

impl<F: FieldExt> EventTableOpcodeConfig<F> for ConstConfig<F> {
    fn opcode(&self, meta: &mut VirtualCells<'_, F>) -> Expression<F> {
        // FIXME
        (constant!(bn_to_field(
            &(BigUint::from(OpcodeClass::Const as u64) << OPCODE_CLASS_SHIFT)
        )) + curr!(meta, self.vtype)
            * constant!(bn_to_field(
                &(BigUint::from(1u64) << OPCODE_CONST_VTYPE_SHIFT)
            ))
            + curr!(meta, self.value))
            * curr!(meta, self.enable)
    }

    fn sp_diff(&self, meta: &mut VirtualCells<'_, F>) -> Expression<F> {
        constant_from!(1u64) * curr!(meta, self.enable)
    }
}

#[cfg(test)]
mod tests {
    use crate::test::test_circuit_builder::run_test_circuit;
    use halo2_proofs::pairing::bn256::Fr as Fp;
    use wasmi::{ImportsBuilder, ModuleInstance};

    #[test]
    fn test_ok() {
        let wasm_binary: Vec<u8> = wabt::wat2wasm(
            r#"
                (module
                    (func (export "test")
                      (i32.const 0)
                      (drop)
                    )
                   )
                "#,
        )
        .expect("failed to parse wat");

        let module = wasmi::Module::from_buffer(&wasm_binary).expect("failed to load wasm");

        let instance = ModuleInstance::new(&module, &ImportsBuilder::default())
            .expect("failed to instantiate wasm module")
            .assert_no_start();

        run_test_circuit::<Fp>(&instance).expect("failed")
    }
}
