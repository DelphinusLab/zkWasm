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

use num_traits::ToPrimitive;
use specs::itable::{BinSignedOp, OPCODE_ARG1_SHIFT};
use specs::mtable::VarType;
use specs::step::StepInfo;
use specs::{
    etable::EventTableEntry,
    itable::{OpcodeClass, OPCODE_ARG0_SHIFT, OPCODE_CLASS_SHIFT},
};

pub struct BinSignedConfig {
    lhs: U64Cell,
    rhs: U64Cell,
    res: U64Cell,
    pad: U64OnU8Cell, // used to prove a > b by a = b + pad
    r: U64OnU8Cell, // remainder for div_s quotient for rem_s
    vtype: CommonRangeCell,
    mode: BitCell, // 0 for div_s 1 for rem_s
    bit_mode: BitCell, //0 for 32, 1 for 64
    lhs_sig_bit: BitCell,
    rhs_sig_bit: BitCell,
    rem_is_zero: BitCell,
    lhs_abs: U64OnU8Cell,
    rhs_abs: U64OnU8Cell,
    //TODO: add constraints between vtype and is_32bits, is_64bits
    lookup_stack_read_lhs: MTableLookupCell,
    lookup_stack_read_rhs: MTableLookupCell,
    lookup_stack_write: MTableLookupCell,
}

pub struct BinSignedConfigBuilder {}

/* Common binary operation that isomorphic to unsigned binary arith */

impl<F: FieldExt> EventTableOpcodeConfigBuilder<F> for BinSignedConfigBuilder {
    fn configure(
        common: &mut EventTableCellAllocator<F>,
        constraint_builder: &mut ConstraintBuilder<F>,
    ) -> Box<dyn EventTableOpcodeConfig<F>> {
        let lhs = common.alloc_u64();
        let rhs = common.alloc_u64();
        let res = common.alloc_u64();
        let pad = common.alloc_u64_on_u8();
        let lhs_abs = common.alloc_u64_on_u8();
        let rhs_abs = common.alloc_u64_on_u8();
        let r = common.alloc_u64_on_u8(); // remainder for div_s quotient for rem_s

        let vtype = common.alloc_common_range_value();
        let mode = common.alloc_bit_value();
        let bit_mode = common.alloc_bit_value();

        let rem_is_zero = common.alloc_bit_value();
        let lhs_sig_bit = common.alloc_bit_value();
        let rhs_sig_bit = common.alloc_bit_value();

        constraint_builder.push(
            "div_s constraints",
            Box::new(move |meta| {
                let bound = constant!(bn_to_field(&(BigUint::from(1u64) << 32usize)))
                    * (constant_from!(1) - bit_mode.expr(meta))
                    + constant!(bn_to_field(&(BigUint::from(1u64) << 64usize)))
                        * bit_mode.expr(meta);
                // xor(a,b) = a(1-b) + b(1-a)
                let res_sig_bit = (constant_from!(1) - lhs_sig_bit.expr(meta)) * rhs_sig_bit.expr(meta)
                    + (constant_from!(1) - rhs_sig_bit.expr(meta)) * lhs_sig_bit.expr(meta);
                let res_abs = res.expr(meta) * (constant_from!(1) - res_sig_bit.clone())
                    + (bound - res.expr(meta)) * res_sig_bit;
                let op_div = constant_from!(1) - mode.expr(meta);
                vec![
                    (lhs_abs.expr(meta)
                        - res_abs * rhs_abs.expr(meta)
                        - r.expr(meta))
                        * op_div.clone(),
                    (rhs_abs.expr(meta) - pad.expr(meta) - r.expr(meta)) * op_div,
                ]
            }),
        );

        constraint_builder.push(
            "rem_s constraints",
            Box::new(move |meta| {
                let bound = constant!(bn_to_field(&(BigUint::from(1u64) << 32usize)))
                    * (constant_from!(1) - bit_mode.expr(meta))
                    + constant!(bn_to_field(&(BigUint::from(1u64) << 64usize)))
                        * bit_mode.expr(meta);

                // According to the spec result's sign is equal to lhs's sign if remainder_abs is
                // not equal to 0
                let remainder = res.expr(meta) * (constant_from!(1) - lhs_sig_bit.expr(meta))
                    + (bound - res.expr(meta)) * lhs_sig_bit.expr(meta) * (constant_from!(1) - rem_is_zero.expr(meta));
                let res_abs = r.expr(meta);
                let op_rem = mode.expr(meta);
                vec![
                    (lhs_abs.expr(meta)
                        - res_abs * rhs_abs.expr(meta)
                        - remainder.clone())
                        * op_rem.clone(), // when res not equal to zero
                    (rhs_abs.expr(meta) - pad.expr(meta) - remainder) * op_rem,
                ]
            }),
        );



        let lookup_stack_read_lhs = common.alloc_mtable_lookup();
        let lookup_stack_read_rhs = common.alloc_mtable_lookup();
        let lookup_stack_write = common.alloc_mtable_lookup();

        Box::new(BinSignedConfig {
            lhs,
            rhs,
            res,
            pad,
            vtype,
            lhs_abs,
            rhs_abs,
            lhs_sig_bit,
            rhs_sig_bit,
            mode,
            rem_is_zero,
            bit_mode,
            r,
            lookup_stack_read_lhs,
            lookup_stack_read_rhs,
            lookup_stack_write,
        })
    }
}

impl<F: FieldExt> EventTableOpcodeConfig<F> for BinSignedConfig {
    fn opcode(&self, meta: &mut VirtualCells<'_, F>) -> Expression<F> {
        constant!(bn_to_field(
            &(BigUint::from(OpcodeClass::BinSigned as u64) << OPCODE_CLASS_SHIFT)
        )) + (self.mode.expr(meta))
            * constant!(bn_to_field(&(BigUint::from(1u64) << OPCODE_ARG0_SHIFT)))
            + self.vtype.expr(meta)
                * constant!(bn_to_field(&(BigUint::from(1u64) << OPCODE_ARG1_SHIFT)))
    }

    fn assign(
        &self,
        ctx: &mut Context<'_, F>,
        step_info: &StepStatus,
        entry: &EventTableEntry,
    ) -> Result<(), Error> {
        let (class, vtype, shift, left, right, value) = match &entry.step_info {
            StepInfo::I32BinSignedOp {
                class,
                left,
                right,
                value,
            } => {
                let vtype = VarType::I32;
                let left = *left as u32 as u64;
                let right = *right as u32 as u64;
                let value = *value as u32 as u64;

                (class, vtype, 32, left, right, value)
            }

            StepInfo::I64BinSignedOp {
                class,
                left,
                right,
                value,
            } => {
                let vtype = VarType::I64;
                let left = *left as u64;
                let right = *right as u64;
                let value = *value as u64;

                (class, vtype, 64, left, right, value)
            }

            _ => unreachable!(),
        };

        self.vtype.assign(ctx, vtype as u16)?;
        self.lhs.assign(ctx, left)?;
        self.rhs.assign(ctx, right)?;
        self.res.assign(ctx, value)?;

        let mut lhs_abs = left;
        let mut rhs_abs = right;

        let lhs_sig_bit = (left >> (shift-1)) & 1 == 1;
        let rhs_sig_bit = (right >> (shift-1)) & 1 == 1;

        if (left >> (shift-1)) & 1 == 1 {
            lhs_abs =  ((BigUint::from(1u64) << shift as u64) - BigUint::from(left)).to_u64().unwrap();
        }
        if (right >> (shift-1)) & 1 == 1 {
            rhs_abs =  ((BigUint::from(1u64) << shift as u64) - BigUint::from(right)).to_u64().unwrap();
        }

        self.lhs_sig_bit.assign(ctx, lhs_sig_bit)?;
        self.rhs_sig_bit.assign(ctx, rhs_sig_bit)?;

        match vtype {
            VarType::I32 => {
                self.bit_mode.assign(ctx, false)?;
            },
            VarType::I64 => {
                self.bit_mode.assign(ctx, true)?;
            }

        };

        self.lhs_abs.assign(ctx, lhs_abs)?;
        self.rhs_abs.assign(ctx, rhs_abs)?;

        let res_abs = lhs_abs / rhs_abs;


        match class {
            specs::itable::BinSignedOp::DivS=> {
                let pad = (res_abs + 1) * rhs_abs - lhs_abs;
                let r = lhs_abs - res_abs * rhs_abs;
                self.mode.assign(ctx, false)?;
                self.pad.assign(ctx, pad)?;
                self.r.assign(ctx, r)?;
                self.rem_is_zero.assign(ctx, rhs_abs * res_abs == lhs_abs)?;
                println!("lhs_abs: {}[{}], rhs_abs: {}[{}], res[{}]: {}, pad: {}, r: {}", lhs_abs, lhs_sig_bit, rhs_abs, rhs_sig_bit, res_abs, value, pad, r);
            },
            specs::itable::BinSignedOp::RemS => {
                let pad = (res_abs + 1) * rhs_abs - lhs_abs;
                let r = res_abs;
                self.mode.assign(ctx, true)?;
                self.pad.assign(ctx, (res_abs + 1) * rhs_abs - lhs_abs)?;
                self.r.assign(ctx, res_abs)?;
                self.rem_is_zero.assign(ctx, rhs_abs * res_abs == lhs_abs)?;
                println!("lhs_abs: {}[{}], rhs_abs: {}[{}], res[{}]: {}, pad: {}, r: {}", lhs_abs, lhs_sig_bit, rhs_abs, rhs_sig_bit, res_abs, value, pad, r);
            },

        };


        self.lookup_stack_read_lhs.assign(
            ctx,
            &MemoryTableLookupEncode::encode_stack_read(
                BigUint::from(step_info.current.eid),
                BigUint::from(1 as u64),
                BigUint::from(step_info.current.sp + 1),
                BigUint::from(vtype as u16),
                BigUint::from(right),
            ),
        )?;

        self.lookup_stack_read_rhs.assign(
            ctx,
            &MemoryTableLookupEncode::encode_stack_read(
                BigUint::from(step_info.current.eid),
                BigUint::from(2 as u64),
                BigUint::from(step_info.current.sp + 2),
                BigUint::from(vtype as u16),
                BigUint::from(left),
            ),
        )?;

        self.lookup_stack_write.assign(
            ctx,
            &MemoryTableLookupEncode::encode_stack_write(
                BigUint::from(step_info.current.eid),
                BigUint::from(3 as u64),
                BigUint::from(step_info.current.sp + 2),
                BigUint::from(vtype as u16),
                BigUint::from(value),
            ),
        )?;

        Ok(())
    }

    fn opcode_class(&self) -> OpcodeClass {
        OpcodeClass::BinSigned
    }

    fn mops(&self, _meta: &mut VirtualCells<'_, F>) -> Option<Expression<F>> {
        Some(constant_from!(3))
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
                self.rhs.expr(meta),
            )),
            MLookupItem::Second => Some(MemoryTableLookupEncode::encode_stack_read(
                common_config.eid(meta),
                constant_from!(2),
                common_config.sp(meta) + constant_from!(2),
                self.vtype.expr(meta),
                self.lhs.expr(meta),
            )),
            MLookupItem::Third => Some(MemoryTableLookupEncode::encode_stack_write(
                common_config.eid(meta),
                constant_from!(3),
                common_config.sp(meta) + constant_from!(2),
                self.vtype.expr(meta),
                self.res.expr(meta),
            )),
            _ => None,
        }
    }

    fn sp_diff(&self, _meta: &mut VirtualCells<'_, F>) -> Option<Expression<F>> {
        Some(constant!(F::one()))
    }
}

#[cfg(test)]
mod tests {
    use crate::test::test_circuit_noexternal;

    #[test]
    fn test_i32_divs_eq() {
        let textual_repr = r#"
                (module
                    (func (export "test")
                      (i32.const 4)
                      (i32.const 4)
                      i32.div_s
                      drop
                    )
                   )
                "#;

        test_circuit_noexternal(textual_repr).unwrap()
    }

    #[test]
    fn test_i32_divs_neq() {
        let textual_repr = r#"
                (module
                    (func (export "test")
                      (i32.const 4)
                      (i32.const 3)
                      i32.div_s
                      drop
                    )
                   )
                "#;

        test_circuit_noexternal(textual_repr).unwrap()
    }


    #[test]
    fn test_i32_divs_eq_neg_lhs() {
        let textual_repr = r#"
                (module
                    (func (export "test")
                      (i32.const -4)
                      (i32.const 4)
                      i32.div_s
                      drop
                    )
                   )
                "#;

        test_circuit_noexternal(textual_repr).unwrap()
    }

    #[test]
    fn test_i32_divs_neq_neg_lhs() {
        let textual_repr = r#"
                (module
                    (func (export "test")
                      (i32.const -4)
                      (i32.const 3)
                      i32.div_s
                      drop
                    )
                   )
                "#;

        test_circuit_noexternal(textual_repr).unwrap()
    }



    #[test]
    fn test_i32_divs_eq_neg() {
        let textual_repr = r#"
                (module
                    (func (export "test")
                      (i32.const -4)
                      (i32.const -4)
                      i32.div_s
                      drop
                    )
                   )
                "#;

        test_circuit_noexternal(textual_repr).unwrap()
    }


    #[test]
    fn test_i32_divs_neq_neg() {
        let textual_repr = r#"
                (module
                    (func (export "test")
                      (i32.const -4)
                      (i32.const -3)
                      i32.div_s
                      drop
                    )
                   )
                "#;

        test_circuit_noexternal(textual_repr).unwrap()
    }

    #[test]
    fn test_i32_rems_eq() {
        let textual_repr = r#"
                (module
                    (func (export "test")
                      (i32.const 4)
                      (i32.const 4)
                      i32.rem_s
                      drop
                    )
                   )
                "#;

        test_circuit_noexternal(textual_repr).unwrap()
    }

    #[test]
    fn test_i32_rems_eq_neg() {
        let textual_repr = r#"
                (module
                    (func (export "test")
                      (i32.const -4)
                      (i32.const -4)
                      i32.rem_s
                      drop
                    )
                   )
                "#;

        test_circuit_noexternal(textual_repr).unwrap()
    }



    #[test]
    fn test_i32_rems_neq() {
        let textual_repr = r#"
                (module
                    (func (export "test")
                      (i32.const 4)
                      (i32.const 3)
                      i32.rem_s
                      drop
                    )
                   )
                "#;

        test_circuit_noexternal(textual_repr).unwrap()
    }


    #[test]
    fn test_i32_rems_neq_neg_1() {
        let textual_repr = r#"
                (module
                    (func (export "test")
                      (i32.const -4)
                      (i32.const -3)
                      i32.rem_s
                      drop
                    )
                   )
                "#;

        test_circuit_noexternal(textual_repr).unwrap()
    }

    #[test]
    fn test_i32_rems_neq_neg_2() {
        let textual_repr = r#"
                (module
                    (func (export "test")
                      (i32.const 4)
                      (i32.const -3)
                      i32.rem_s
                      drop
                    )
                   )
                "#;

        test_circuit_noexternal(textual_repr).unwrap()
    }

    #[test]
    fn test_i32_rems_neq_neg_3() {
        let textual_repr = r#"
                (module
                    (func (export "test")
                      (i32.const -4)
                      (i32.const 3)
                      i32.rem_s
                      drop
                    )
                   )
                "#;

        test_circuit_noexternal(textual_repr).unwrap()
    }

    #[test]
    fn test_i64_divs_eq() {
        let textual_repr = r#"
                (module
                    (func (export "test")
                      (i64.const 4)
                      (i64.const 4)
                      i64.div_s
                      drop
                    )
                   )
                "#;

        test_circuit_noexternal(textual_repr).unwrap()
    }

    #[test]
    fn test_i64_divs_neq() {
        let textual_repr = r#"
                (module
                    (func (export "test")
                      (i64.const 4)
                      (i64.const 3)
                      i64.div_s
                      drop
                    )
                   )
                "#;

        test_circuit_noexternal(textual_repr).unwrap()
    }


    #[test]
    fn test_i64_divs_eq_neg_lhs() {
        let textual_repr = r#"
                (module
                    (func (export "test")
                      (i64.const -4)
                      (i64.const 4)
                      i64.div_s
                      drop
                    )
                   )
                "#;

        test_circuit_noexternal(textual_repr).unwrap()
    }

    #[test]
    fn test_i64_divs_neq_neg_lhs() {
        let textual_repr = r#"
                (module
                    (func (export "test")
                      (i64.const -4)
                      (i64.const 3)
                      i64.div_s
                      drop
                    )
                   )
                "#;

        test_circuit_noexternal(textual_repr).unwrap()
    }



    #[test]
    fn test_i64_divs_eq_neg() {
        let textual_repr = r#"
                (module
                    (func (export "test")
                      (i64.const -4)
                      (i64.const -4)
                      i64.div_s
                      drop
                    )
                   )
                "#;

        test_circuit_noexternal(textual_repr).unwrap()
    }


    #[test]
    fn test_i64_divs_neq_neg() {
        let textual_repr = r#"
                (module
                    (func (export "test")
                      (i64.const -4)
                      (i64.const -3)
                      i64.div_s
                      drop
                    )
                   )
                "#;

        test_circuit_noexternal(textual_repr).unwrap()
    }

    #[test]
    fn test_i64_rems_eq() {
        let textual_repr = r#"
                (module
                    (func (export "test")
                      (i64.const 4)
                      (i64.const 4)
                      i64.rem_s
                      drop
                    )
                   )
                "#;

        test_circuit_noexternal(textual_repr).unwrap()
    }

    #[test]
    fn test_i64_rems_eq_neg() {
        let textual_repr = r#"
                (module
                    (func (export "test")
                      (i64.const -4)
                      (i64.const -4)
                      i64.rem_s
                      drop
                    )
                   )
                "#;

        test_circuit_noexternal(textual_repr).unwrap()
    }



    #[test]
    fn test_i64_rems_neq() {
        let textual_repr = r#"
                (module
                    (func (export "test")
                      (i64.const 4)
                      (i64.const 3)
                      i64.rem_s
                      drop
                    )
                   )
                "#;

        test_circuit_noexternal(textual_repr).unwrap()
    }


    #[test]
    fn test_i64_rems_neq_neg_1() {
        let textual_repr = r#"
                (module
                    (func (export "test")
                      (i64.const -4)
                      (i64.const -3)
                      i64.rem_s
                      drop
                    )
                   )
                "#;

        test_circuit_noexternal(textual_repr).unwrap()
    }

    #[test]
    fn test_i64_rems_neq_neg_2() {
        let textual_repr = r#"
                (module
                    (func (export "test")
                      (i64.const 4)
                      (i64.const -3)
                      i64.rem_s
                      drop
                    )
                   )
                "#;

        test_circuit_noexternal(textual_repr).unwrap()
    }

    #[test]
    fn test_i64_rems_neq_neg_3() {
        let textual_repr = r#"
                (module
                    (func (export "test")
                      (i64.const -4)
                      (i64.const 3)
                      i64.rem_s
                      drop
                    )
                   )
                "#;

        test_circuit_noexternal(textual_repr).unwrap()
    }

}
