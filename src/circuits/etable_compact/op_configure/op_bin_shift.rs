use super::*;
use crate::{
    circuits::{
        mtable_compact::encode::MemoryTableLookupEncode,
        rtable::pow_table_encode,
        utils::{bn_to_field, Context},
    },
    constant,
};
use halo2_proofs::{
    arithmetic::FieldExt,
    plonk::{Error, Expression, VirtualCells},
};
use specs::itable::{ShiftOp, OPCODE_ARG1_SHIFT};
use specs::mtable::VarType;
use specs::step::StepInfo;
use specs::{
    etable::EventTableEntry,
    itable::{OpcodeClass, OPCODE_ARG0_SHIFT, OPCODE_CLASS_SHIFT},
};

pub struct BinShiftConfig {
    lhs: U64Cell,
    rhs: U64Cell,
    modulus: U64Cell, // modulus = 2 ^ (rhs % REM_OF_SIZE)
    round: U64OnU8Cell,
    rem: U64OnU8Cell,  // round * x + rem = y
    diff: U64OnU8Cell, // to limit the rem range
    pad:U64OnU8Cell, // the padding part when doing signed op 
    res: UnlimitedCell,

    is_eight_bytes: BitCell,
    higher_u4_decompose: [BitCell; 4],

    is_shl: BitCell,
    is_shr_u: BitCell,
    is_shr_s:BitCell,
    is_neg: BitCell,
    lookup_pow: PowTableLookupCell,

    lookup_stack_read_lhs: MTableLookupCell,
    lookup_stack_read_rhs: MTableLookupCell,
    lookup_stack_write: MTableLookupCell,
}

pub struct BinShiftConfigBuilder {}

impl<F: FieldExt> EventTableOpcodeConfigBuilder<F> for BinShiftConfigBuilder {
    fn configure(
        common: &mut EventTableCellAllocator<F>,
        constraint_builder: &mut ConstraintBuilder<F>,
    ) -> Box<dyn EventTableOpcodeConfig<F>> {
        let lhs = common.alloc_u64();
        let rhs = common.alloc_u64();
        let modulus = common.alloc_u64();
        let round = common.alloc_u64_on_u8();
        let diff = common.alloc_u64_on_u8();
        let rem = common.alloc_u64_on_u8();
        let res = common.alloc_unlimited_value();

        let is_eight_bytes = common.alloc_bit_value();
        let higher_u4_decompose = [0; 4].map(|_| common.alloc_bit_value());

        let is_shl = common.alloc_bit_value();
        let is_shr_u = common.alloc_bit_value();
        let is_shr_s = common.alloc_bit_value();
        let is_neg:BitCell= common.alloc_bit_value();
        let pad = common.alloc_u64_on_u8();
        let lookup_pow = common.alloc_pow_table_lookup();

        let lookup_stack_read_lhs = common.alloc_mtable_lookup();
        let lookup_stack_read_rhs = common.alloc_mtable_lookup();
        let lookup_stack_write = common.alloc_mtable_lookup();

        constraint_builder.push(
            "bin decompose u4",
            Box::new(move |meta| {
                vec![higher_u4_decompose
                    .iter()
                    .enumerate()
                    .fold(rhs.u4_expr(meta, 1), |acc, (pos, cell)| {
                        acc - cell.expr(meta) * constant_from!(1 << pos)
                    })]
            }),
        );

        constraint_builder.push(
            "bin pow lookup",
            Box::new(move |meta| {
                let power = is_eight_bytes.expr(meta)
                    * higher_u4_decompose[1].clone().expr(meta)
                    * constant_from!(1 << 5)
                    + higher_u4_decompose[0].clone().expr(meta) * constant_from!(1 << 4)
                    + rhs.u4_expr(meta, 0);
                vec![lookup_pow.expr(meta) - pow_table_encode(modulus.expr(meta), power)]
            }),
        );

        constraint_builder.push(
            "bin shr_u",
            Box::new(move |meta| {
                vec![
                    is_shr_u.expr(meta) * (rem.expr(meta) + diff.expr(meta) - modulus.expr(meta)),
                    is_shr_u.expr(meta)
                        * (rem.expr(meta) + round.expr(meta) * modulus.expr(meta) - lhs.expr(meta)),
                    is_shr_u.expr(meta) * (res.expr(meta) - round.expr(meta)),
                ]
            }),
        );
        constraint_builder.push(
            "bin shr_s",
            Box::new(move |meta| {
                vec![
                    is_shr_s.expr(meta)
                        * (rem.expr(meta) + round.expr(meta) * modulus.expr(meta) - lhs.expr(meta)),
                    is_shr_s.expr(meta)
                        * (rem.expr(meta) + round.expr(meta) * modulus.expr(meta) - lhs.expr(meta)),
                    is_shr_s.expr(meta) * (res.expr(meta) - round.expr(meta))
                        *(constant_from!(1) - is_neg.expr(meta)),
                    is_shr_s.expr(meta) 
                        * (res.expr(meta) - round.expr(meta)- pad.expr(meta))
                        *(is_neg.expr(meta)),
                    is_shr_s.expr(meta) *(pad.expr(meta)*modulus.expr(meta)+constant_from!(1u64<<32)-
                    constant_from!(1u64<<32)*modulus.expr(meta))
                    *(is_neg.expr(meta)),
                    
                ]
            }),
        );

        constraint_builder.push(
            "bin shl",
            Box::new(move |meta| {
                vec![
                    // is u64
                    is_shl.expr(meta)
                        * is_eight_bytes.expr(meta)
                        * (lhs.expr(meta) * modulus.expr(meta)
                            - round.expr(meta)
                                * constant!(bn_to_field::<F>(&(BigUint::from(1u64) << 64)))
                            - rem.expr(meta)),
                    // is u32
                    is_shl.expr(meta)
                        * (constant_from!(1) - is_eight_bytes.expr(meta))
                        * (lhs.expr(meta) * modulus.expr(meta)
                            - round.expr(meta) * constant_from!(1u64 << 32)
                            - rem.expr(meta)),
                    is_shl.expr(meta)
                        * (constant_from!(1) - is_eight_bytes.expr(meta))
                        * (rem.expr(meta) + diff.expr(meta) - constant_from!(1u64 << 32)),
                    // res
                    is_shl.expr(meta) * (res.expr(meta) - rem.expr(meta)),
                ]
            }),
        );
     

        Box::new(BinShiftConfig {
            lhs,
            rhs,
            modulus,
            round,
            rem,
            res,
            diff,
            is_eight_bytes,
            higher_u4_decompose,
            is_shl,
            is_shr_u,
            is_shr_s,
            is_neg,
            pad,
            lookup_stack_read_lhs,
            lookup_stack_read_rhs,
            lookup_stack_write,
            lookup_pow,
        })
    }
}

impl<F: FieldExt> EventTableOpcodeConfig<F> for BinShiftConfig {
    fn opcode(&self, meta: &mut VirtualCells<'_, F>) -> Expression<F> {
        let vtype = self.is_eight_bytes.expr(meta) + constant_from!(1);

        constant!(bn_to_field(
            &(BigUint::from(OpcodeClass::BinShift as u64) << OPCODE_CLASS_SHIFT)
        )) + self.is_shl.expr(meta)
            * constant!(bn_to_field(
                &(BigUint::from(ShiftOp::Shl as u64) << OPCODE_ARG0_SHIFT)
            ))
            + self.is_shr_u.expr(meta)
                * constant!(bn_to_field(
                    &(BigUint::from(ShiftOp::UnsignedShr as u64) << OPCODE_ARG0_SHIFT)
                ))
                + self.is_shr_s.expr(meta)
                * constant!(bn_to_field(
                    &(BigUint::from(ShiftOp::UnsignedShr as u64) << OPCODE_ARG0_SHIFT)
                ))
            + vtype * constant!(bn_to_field(&(BigUint::from(1u64) << OPCODE_ARG1_SHIFT)))
    }

    fn assign(
        &self,
        ctx: &mut Context<'_, F>,
        step_info: &StepStatus,
        entry: &EventTableEntry,
    ) -> Result<(), Error> {
        let (class, vtype, left, right, value, power, is_eight_bytes, is_sign) =
            match entry.step_info {
                StepInfo::I32BinShiftOp {
                    class,
                    left,
                    right,
                    value,
                } => {
                    let vtype = VarType::I32;
                    let left = left as u32 as u64;
                    let right = right as u32 as u64;
                    let value = value as u32 as u64;
                    let power = right % 32;
                    let is_eight_bytes = false;
                    let is_sign = true;
                    (
                        class,
                        vtype,
                        left,
                        right,
                        value,
                        power,
                        is_eight_bytes,
                        is_sign,
                    )
                }

                _ => unreachable!(),
            };

        self.lhs.assign(ctx, left)?;
        self.rhs.assign(ctx, right)?;
        self.modulus.assign(ctx, 1 << power)?;
        self.lookup_pow.assign(ctx, power)?;
        self.is_eight_bytes.assign(ctx, is_eight_bytes)?;
        self.res.assign(ctx, F::from(value))?;

        for i in 0..4 {
            self.higher_u4_decompose[i].assign(ctx, (right >> (4 + i)) & 1 == 1)?;
        }

        match class {
            ShiftOp::Shl => {
                self.is_shl.assign(ctx, true)?;
                self.round.assign(ctx, left >> (32 - power))?;
                let rem = (left << power) & ((1u64 << 32) - 1);
                self.rem.assign(ctx, rem)?;
                self.diff.assign(ctx, (1u64 << 32) - rem)?;
            }
            ShiftOp::UnsignedShr => {
                self.is_shr_u.assign(ctx, true)?;
                self.round.assign(ctx, left >> power)?;
                let rem = left & ((1 << power) - 1);
                self.rem.assign(ctx, rem)?;
                self.diff.assign(ctx, (1u64 << power) - rem)?;
            }

            ShiftOp::SignedShr =>{
                self.is_shr_s.assign(ctx, true)?;
                match left>>31  {
                    0u64=>{
                        self.is_neg.assign(ctx, false)?;
                        self.round.assign(ctx, left >> power)?;
                        let rem = left & ((1 << power) - 1);
                        self.rem.assign(ctx, rem)?;
                        self.diff.assign(ctx, (1u64 << power) - rem)?;
                        
                    }
                    1u64=>{
                       
                        self.is_neg.assign(ctx, true)?;
                        self.round.assign(ctx, left >> power)?;
                        self.pad.assign(ctx, ((1<<power)-1)<<(32-power))?;
                        let rem = left & ((1 << power) - 1);
                        self.rem.assign(ctx, rem)?;
                        self.diff.assign(ctx, (1u64 << power) -1 - rem)?;
                    }
                    _=>unreachable!()
                }
            }
            ShiftOp::Rotl => todo!(),
        }

        self.lookup_stack_read_lhs.assign(
            ctx,
            &MemoryTableLookupEncode::encode_stack_read(
                BigUint::from(step_info.current.eid),
                BigUint::from(1 as u64),
                BigUint::from(step_info.current.sp + 1),
                BigUint::from(vtype as u64),
                BigUint::from(right),
            ),
        )?;

        self.lookup_stack_read_rhs.assign(
            ctx,
            &MemoryTableLookupEncode::encode_stack_read(
                BigUint::from(step_info.current.eid),
                BigUint::from(2 as u64),
                BigUint::from(step_info.current.sp + 2),
                BigUint::from(vtype as u64),
                BigUint::from(left),
            ),
        )?;

        self.lookup_stack_write.assign(
            ctx,
            &MemoryTableLookupEncode::encode_stack_write(
                BigUint::from(step_info.current.eid),
                BigUint::from(3 as u64),
                BigUint::from(step_info.current.sp + 2),
                BigUint::from(vtype as u64),
                BigUint::from(value),
            ),
        )?;

        Ok(())
    }

    fn opcode_class(&self) -> OpcodeClass {
        OpcodeClass::BinShift
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
        let vtype = self.is_eight_bytes.expr(meta) + constant_from!(1);
        match item {
            MLookupItem::First => Some(MemoryTableLookupEncode::encode_stack_read(
                common_config.eid(meta),
                constant_from!(1),
                common_config.sp(meta) + constant_from!(1),
                vtype.clone(),
                self.rhs.expr(meta),
            )),
            MLookupItem::Second => Some(MemoryTableLookupEncode::encode_stack_read(
                common_config.eid(meta),
                constant_from!(2),
                common_config.sp(meta) + constant_from!(2),
                vtype.clone(),
                self.lhs.expr(meta),
            )),
            MLookupItem::Third => Some(MemoryTableLookupEncode::encode_stack_write(
                common_config.eid(meta),
                constant_from!(3),
                common_config.sp(meta) + constant_from!(2),
                vtype.clone(),
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
    fn test_i32_shr_u_1_ok() {
        let textual_repr = r#"
                (module
                    (func (export "test")
                      (i32.const 12)
                      (i32.const 3)
                      (i32.shr_u)
                      (drop)
                    )
                   )
                "#;

        test_circuit_noexternal(textual_repr).unwrap()
    }

    #[test]
    fn test_i32_shr_u_2_ok() {
        let textual_repr = r#"
                (module
                    (func (export "test")
                      (i32.const 12)
                      (i32.const 35)
                      (i32.shr_u)
                      (drop)
                    )
                   )
                "#;

        test_circuit_noexternal(textual_repr).unwrap()
    }

    #[test]
    fn test_i32_shl_1_ok() {
        let textual_repr = r#"
                (module
                    (func (export "test")
                      (i32.const 12)
                      (i32.const 0)
                      (i32.shl)
                      (drop)
                    )
                   )
                "#;

        test_circuit_noexternal(textual_repr).unwrap()
    }

    #[test]
    fn test_i32_shl_2_ok() {
        let textual_repr = r#"
                (module
                    (func (export "test")
                      (i32.const 12)
                      (i32.const 1)
                      (i32.shl)
                      (drop)
                    )
                   )
                "#;

        test_circuit_noexternal(textual_repr).unwrap()
    }

    #[test]
    fn test_i32_shl_3_ok() {
        let textual_repr = r#"
                (module
                    (func (export "test")
                      (i32.const 12)
                      (i32.const 33)
                      (i32.shl)
                      (drop)
                    )
                   )
                "#;

        test_circuit_noexternal(textual_repr).unwrap()
    }

    #[test]
    fn test_i32_shl_overflow_ok() {
        let textual_repr = r#"
                (module
                    (func (export "test")
                      (i32.const 4294967295)
                      (i32.const 1)
                      (i32.shl)
                      (drop)
                    )
                   )
                "#;

        test_circuit_noexternal(textual_repr).unwrap()
    }

    #[test]
    fn test_i32_shr_s_positive(){
        let textual_repr = r#"
                (module
                    (func (export "test")
                      (i32.const 23)
                      (i32.const 2)
                      (i32.shr_s)
                      (drop)
                    )
                   )
                "#;

        test_circuit_noexternal(textual_repr).unwrap()
    }

    #[test]
    fn test_i32_shr_s_negative(){
        let textual_repr = r#"
                (module
                    (func (export "test")
                      (i32.const -23)
                      (i32.const 5)
                      (i32.shr_s)
                      (drop)
                    )
                   )
                "#;

        test_circuit_noexternal(textual_repr).unwrap()
    }
    #[test]
    fn test_i32_shr_s_zero(){
        let textual_repr = r#"
                (module
                    (func (export "test")
                      (i32.const 0)
                      (i32.const 5)
                      (i32.shr_s)
                      (drop)
                    )
                   )
                "#;

        test_circuit_noexternal(textual_repr).unwrap()
    }
    #[test]
    fn test_i32_shr_s_overflow(){
        let textual_repr = r#"
                (module
                    (func (export "test")
                      (i32.const -1)
                      (i32.const 5)
                      (i32.shr_s)
                      (drop)
                    )
                   )
                "#;

        test_circuit_noexternal(textual_repr).unwrap()
    }
}
