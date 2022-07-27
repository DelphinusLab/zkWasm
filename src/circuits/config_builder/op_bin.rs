use std::vec;

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
    itable::{BinOp, OpcodeClass, OPCODE_ARG0_SHIFT, OPCODE_ARG1_SHIFT, OPCODE_CLASS_SHIFT},
    mtable::VarType,
};

pub struct BinOpConfig<F: FieldExt> {
    left: U64Config<F>,
    right: U64Config<F>,
    res: U64Config<F>,
    overflow: U64Config<F>,
    enable: Column<Advice>,
    is_add: Column<Advice>,
    vtype: Column<Advice>,
    vtype_len_bitmask: [Column<Advice>; 4],
}

pub struct BinOpConfigBuilder {}

impl<F: FieldExt> EventTableOpcodeConfigBuilder<F> for BinOpConfigBuilder {
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
        let is_add = cols.next().unwrap();
        let vtype = cols.next().unwrap();
        let vtype_len_bitmask = [0; 4].map(|_| cols.next().unwrap());

        let overflow = U64Config::configure(meta, cols, rtable, |meta| {
            curr!(meta, opcode_bit) * enable(meta)
        });
        let left = U64Config::configure(meta, cols, rtable, |meta| {
            curr!(meta, opcode_bit) * enable(meta)
        });
        let right = U64Config::configure(meta, cols, rtable, |meta| {
            curr!(meta, opcode_bit) * enable(meta)
        });
        let res = U64Config::configure(meta, cols, rtable, |meta| {
            curr!(meta, opcode_bit) * enable(meta)
        });

        meta.create_gate("is add", |meta| {
            vec![
                curr!(meta, is_add)
                    * (curr!(meta, is_add) - constant_from!(1))
                    * curr!(meta, opcode_bit)
                    * enable(meta),
                (curr!(meta, is_add) - constant_from!(1)) * curr!(meta, opcode_bit) * enable(meta),
            ]
        });

        meta.create_gate("vtype bits", |meta| {
            vtype_len_bitmask
                .iter()
                .map(|c| {
                    curr!(meta, *c)
                        * (curr!(meta, *c) - constant_from!(1))
                        * curr!(meta, opcode_bit)
                        * enable(meta)
                })
                .collect::<Vec<_>>()
        });

        meta.create_gate("vtype sum one", |meta| {
            vec![
                (vtype_len_bitmask
                    .iter()
                    .map(|c| curr!(meta, *c))
                    .reduce(|acc, c| acc + c)
                    .unwrap()
                    - constant_from!(1))
                    * curr!(meta, opcode_bit)
                    * enable(meta),
            ]
        });

        mtable.configure_stack_read_in_table(
            "bin mlookup",
            meta,
            |meta| curr!(meta, opcode_bit) * enable(meta),
            |meta| curr!(meta, common.eid),
            |_meta| constant_from!(1),
            |meta| curr!(meta, common.sp) + constant_from!(1),
            |meta| curr!(meta, vtype),
            |meta| curr!(meta, right.value),
        );

        mtable.configure_stack_read_in_table(
            "bin mlookup",
            meta,
            |meta| curr!(meta, opcode_bit) * enable(meta),
            |meta| curr!(meta, common.eid),
            |_meta| constant_from!(2),
            |meta| curr!(meta, common.sp) + constant_from!(2),
            |meta| curr!(meta, vtype),
            |meta| curr!(meta, left.value),
        );

        mtable.configure_stack_write_in_table(
            "bin mlookup",
            meta,
            |meta| curr!(meta, opcode_bit) * enable(meta),
            |meta| curr!(meta, common.eid),
            |_meta| constant_from!(3),
            |meta| curr!(meta, common.sp) + constant_from!(2),
            |meta| curr!(meta, vtype),
            |meta| curr!(meta, res.value),
        );

        // configure for BinOp
        meta.create_gate("op bin add equation", |meta| {
            let modules = constant_from!(1 << 8usize) * curr!(meta, vtype_len_bitmask[0])
                + constant!(bn_to_field(&(BigUint::from(1u64) << 16usize)))
                    * curr!(meta, vtype_len_bitmask[1])
                + constant!(bn_to_field(&(BigUint::from(1u64) << 32usize)))
                    * curr!(meta, vtype_len_bitmask[2])
                + constant!(bn_to_field(&(BigUint::from(1u64) << 64usize)))
                    * curr!(meta, vtype_len_bitmask[3]);
            vec![
                (curr!(meta, left.value) + curr!(meta, right.value)
                    - curr!(meta, res.value)
                    - curr!(meta, overflow.value) * modules.clone())
                    * enable(meta)
                    * curr!(meta, opcode_bit)
                    * curr!(meta, is_add),
            ]
        });

        // TODO: optimize them by merge the vtype cols into one
        meta.create_gate("op bin vtype constraints", |meta| {
            vec![
                (curr!(meta, vtype)
                    - curr!(meta, vtype_len_bitmask[0]) * constant_from!(VarType::I8)
                    - curr!(meta, vtype_len_bitmask[1]) * constant_from!(VarType::I16)
                    - curr!(meta, vtype_len_bitmask[2]) * constant_from!(VarType::I32)
                    - curr!(meta, vtype_len_bitmask[3]) * constant_from!(VarType::I64))
                    * (curr!(meta, vtype)
                        - curr!(meta, vtype_len_bitmask[0]) * constant_from!(VarType::U8)
                        - curr!(meta, vtype_len_bitmask[1]) * constant_from!(VarType::U16)
                        - curr!(meta, vtype_len_bitmask[2]) * constant_from!(VarType::U32)
                        - curr!(meta, vtype_len_bitmask[3]) * constant_from!(VarType::U64))
                    * enable(meta)
                    * curr!(meta, opcode_bit),
            ]
        });

        Box::new(BinOpConfig {
            is_add,
            vtype_len_bitmask,
            enable: opcode_bit,
            overflow,
            left,
            right,
            res,
            vtype,
        })
    }
}

impl<F: FieldExt> EventTableOpcodeConfig<F> for BinOpConfig<F> {
    fn opcode(&self, meta: &mut VirtualCells<'_, F>) -> Expression<F> {
        (constant!(bn_to_field(
            &(BigUint::from(OpcodeClass::Bin as u64) << OPCODE_CLASS_SHIFT)
        )) + curr!(meta, self.is_add)
            * constant!(bn_to_field(
                &(BigUint::from(BinOp::Add as u64) << OPCODE_ARG0_SHIFT)
            ))
            + curr!(meta, self.vtype)
                * constant!(bn_to_field(&(BigUint::from(1u64) << OPCODE_ARG1_SHIFT))))
            * curr!(meta, self.enable)
    }

    fn sp_diff(&self, meta: &mut VirtualCells<'_, F>) -> Expression<F> {
        constant!(F::one()) * curr!(meta, self.enable)
    }

    fn opcode_class(&self) -> OpcodeClass {
        OpcodeClass::Bin
    }

    fn assign(&self, ctx: &mut Context<'_, F>, entry: &EventTableEntry) -> Result<(), Error> {
        match entry.step_info {
            specs::step::StepInfo::I32BinOp {
                class,
                left,
                right,
                value,
            } => {
                ctx.region.assign_advice(
                    || "bin op i32 vtype",
                    self.vtype,
                    ctx.offset,
                    || Ok(F::from(VarType::I32 as u64)),
                )?;
                ctx.region.assign_advice(
                    || "bin op i32 vtype len bit",
                    self.vtype_len_bitmask[2],
                    ctx.offset,
                    || Ok(F::one()),
                )?;
                self.left.assign(ctx, left as u32 as u64)?;
                self.right.assign(ctx, right as u32 as u64)?;
                self.res.assign(ctx, value as u32 as u64)?;

                match class {
                    BinOp::Add => {
                        ctx.region.assign_advice(
                            || "bin op i32 add ",
                            self.is_add,
                            ctx.offset,
                            || Ok(F::one()),
                        )?;
                        self.overflow
                            .assign(ctx, (left as u32 as u64 + right as u32 as u64) >> 32)?;
                    }
                }
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
    use wasmi::{ImportsBuilder, NopExternals};

    #[test]
    fn test_i32_add_ok() {
        let textual_repr = r#"
                (module
                    (func (export "test")
                      (i32.const 1)
                      (i32.const 2)
                      (i32.add)
                      (drop)
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
}
