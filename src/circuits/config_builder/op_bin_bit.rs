use std::vec;

use crate::{
    circuits::{
        etable::{EventTableCommonConfig, EventTableOpcodeConfig, EventTableOpcodeConfigBuilder},
        itable::InstructionTableConfig,
        jtable::JumpTableConfig,
        mtable::MemoryTableConfig,
        rtable::RangeTableConfig,
        utils::{
            bitvalue::BitValueConfig, bn_to_field, tvalue::TValueConfig, u64::U64Config, Context,
        },
    },
    constant, constant_from, curr, fixed_curr,
};
use halo2_proofs::{
    arithmetic::FieldExt,
    plonk::{Advice, Column, ConstraintSystem, Error, Expression, Fixed, VirtualCells},
};
use num_bigint::BigUint;
use specs::{
    etable::EventTableEntry,
    itable::{BitOp, OpcodeClass, OPCODE_ARG0_SHIFT, OPCODE_ARG1_SHIFT, OPCODE_CLASS_SHIFT},
    mtable::VarType,
};

pub struct BinBitOpConfig<F: FieldExt> {
    left: BitValueConfig<F>,
    right: BitValueConfig<F>,
    res: BitValueConfig<F>,
    enable: Column<Advice>,
    is_or: Column<Advice>,
    op: Column<Advice>, //0: or, 1: and, ...
    vtype_len_bitmask: [Column<Advice>; 4],
}

pub struct BinBitOpConfigBuilder {}

impl<F: FieldExt> EventTableOpcodeConfigBuilder<F> for BinBitOpConfigBuilder {
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
        let op = cols.next().unwrap();
        let is_or = cols.next().unwrap();
        let vtype_len_bitmask = [0; 4].map(|_| cols.next().unwrap());

        let left = BitValueConfig::configure(meta, cols, rtable, |meta| {
            curr!(meta, opcode_bit) * enable(meta)
        });
        let right = BitValueConfig::configure(meta, cols, rtable, |meta| {
            curr!(meta, opcode_bit) * enable(meta)
        });
        let res = BitValueConfig::configure(meta, cols, rtable, |meta| {
            curr!(meta, opcode_bit) * enable(meta)
        });

        meta.create_gate("op meets is_xx", |meta| {
            vec![curr!(meta, is_or) * curr!(meta, op) * curr!(meta, opcode_bit) * enable(meta)]
        });

        meta.create_gate("is or", |meta| {
            vec![
                curr!(meta, is_or)
                    * (curr!(meta, is_or) - constant_from!(1))
                    * curr!(meta, opcode_bit)
                    * enable(meta),
                (curr!(meta, is_or) - constant_from!(1)) * curr!(meta, opcode_bit) * enable(meta),
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
            |meta| curr!(meta, right.vtype),
            |meta| curr!(meta, right.value),
        );

        mtable.configure_stack_read_in_table(
            "bin mlookup",
            meta,
            |meta| curr!(meta, opcode_bit) * enable(meta),
            |meta| curr!(meta, common.eid),
            |_meta| constant_from!(2),
            |meta| curr!(meta, common.sp) + constant_from!(2),
            |meta| curr!(meta, left.vtype),
            |meta| curr!(meta, left.value),
        );

        mtable.configure_stack_write_in_table(
            "bin mlookup",
            meta,
            |meta| curr!(meta, opcode_bit) * enable(meta),
            |meta| curr!(meta, common.eid),
            |_meta| constant_from!(3),
            |meta| curr!(meta, common.sp) + constant_from!(2),
            |meta| curr!(meta, res.vtype),
            |meta| curr!(meta, res.value),
        );

        rtable.configure_in_bitop(
            meta,
            "or bit table lookup",
            |meta| curr!(meta, opcode_bit) * enable(meta),
            |meta| curr!(meta, op),
            |meta| curr!(meta, left.value),
            |meta| curr!(meta, right.value),
            |meta| curr!(meta, res.value),
        );

        // TODO: optimize them by merge the vtype cols into one
        meta.create_gate("op bin vtype constrains", |meta| {
            vec![
                (curr!(meta, res.vtype) - curr!(meta, left.vtype)) * enable(meta),
                (curr!(meta, res.vtype) - curr!(meta, right.vtype)) * enable(meta),
                (curr!(meta, res.vtype)
                    - curr!(meta, vtype_len_bitmask[0]) * constant_from!(VarType::I8)
                    - curr!(meta, vtype_len_bitmask[1]) * constant_from!(VarType::I16)
                    - curr!(meta, vtype_len_bitmask[2]) * constant_from!(VarType::I32)
                    - curr!(meta, vtype_len_bitmask[3]) * constant_from!(VarType::I64))
                    * (curr!(meta, res.vtype)
                        - curr!(meta, vtype_len_bitmask[0]) * constant_from!(VarType::U8)
                        - curr!(meta, vtype_len_bitmask[1]) * constant_from!(VarType::U16)
                        - curr!(meta, vtype_len_bitmask[2]) * constant_from!(VarType::U32)
                        - curr!(meta, vtype_len_bitmask[3]) * constant_from!(VarType::U64))
                    * enable(meta)
                    * curr!(meta, opcode_bit),
            ]
        });

        Box::new(BinBitOpConfig {
            is_or,
            vtype_len_bitmask,
            enable: opcode_bit,
            left,
            right,
            res,
            op,
        })
    }
}

impl<F: FieldExt> EventTableOpcodeConfig<F> for BinBitOpConfig<F> {
    fn opcode(&self, meta: &mut VirtualCells<'_, F>) -> Expression<F> {
        (constant!(bn_to_field(
            &(BigUint::from(OpcodeClass::BinBit as u64) << OPCODE_CLASS_SHIFT)
        )) + curr!(meta, self.is_or)
            * constant!(bn_to_field(
                &(BigUint::from(BitOp::Or as u64) << OPCODE_ARG0_SHIFT)
            ))
            + curr!(meta, self.res.vtype)
                * constant!(bn_to_field(&(BigUint::from(1u64) << OPCODE_ARG1_SHIFT))))
            * curr!(meta, self.enable)
    }

    fn sp_diff(&self, meta: &mut VirtualCells<'_, F>) -> Expression<F> {
        constant!(F::one()) * curr!(meta, self.enable)
    }

    fn opcode_class(&self) -> OpcodeClass {
        OpcodeClass::BinBit
    }

    fn assign(&self, ctx: &mut Context<'_, F>, entry: &EventTableEntry) -> Result<(), Error> {
        match entry.step_info {
            specs::step::StepInfo::I32BinBitOp {
                class,
                left,
                right,
                value,
            } => {
                ctx.region.assign_advice(
                    || "bin op i32 vtype len bit",
                    self.vtype_len_bitmask[2],
                    ctx.offset,
                    || Ok(F::one()),
                )?;
                self.left.assign(ctx, VarType::I32, left as u32 as u64)?;
                self.right.assign(ctx, VarType::I32, right as u32 as u64)?;
                self.res.assign(ctx, VarType::I32, value as u32 as u64)?;

                match class {
                    BitOp::Or => {
                        ctx.region.assign_advice(
                            || "bin op i32 or ",
                            self.is_or,
                            ctx.offset,
                            || Ok(F::one()),
                        )?;

                        ctx.region.assign_advice(
                            || "bin op i32 or ",
                            self.op,
                            ctx.offset,
                            || Ok(F::zero()),
                        )?;
                    }
                    _ => todo!(),
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
    use specs::write_json;

    #[test]
    fn test_i32_or_ok() {
        let textual_repr = r#"
                (module
                    (func (export "test")
                      (i32.const 1)
                      (i32.const 2)
                      (i32.or)
                      (drop)
                    )
                   )
                "#;

        let compiler = WasmInterpreter::new();
        let compiled_module = compiler.compile(textual_repr).unwrap();
        let execution_log = compiler.run(&compiled_module, "test", vec![]).unwrap();
        write_json(&compiled_module.tables, &execution_log.tables);
        run_test_circuit::<Fp>(compiled_module.tables, execution_log.tables).unwrap()
    }
}
