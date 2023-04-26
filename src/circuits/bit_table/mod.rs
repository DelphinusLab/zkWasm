use std::marker::PhantomData;

use ark_std::One;
use halo2_proofs::arithmetic::FieldExt;
use halo2_proofs::plonk::Advice;
use halo2_proofs::plonk::Column;
use halo2_proofs::plonk::ConstraintSystem;
use halo2_proofs::plonk::Fixed;
use num_bigint::BigUint;
use specs::encode::FromBn;
use specs::itable::BitOp;
use strum::IntoEnumIterator;

use crate::constant_from;
use crate::curr;
use crate::fixed_curr;
use crate::fixed_nextn;
use crate::next;
use crate::nextn;

use super::config::max_bit_table_rows;
use super::rtable::RangeTableConfig;

mod assign;
mod configure;

const STEP_SIZE: usize = 17;

#[derive(Clone, Copy)]
pub(crate) enum BitTableOp {
    BinaryBit(BitOp),
    Popcnt,
}

impl BitTableOp {
    pub(crate) fn index(&self) -> usize {
        match self {
            BitTableOp::BinaryBit(op) => *op as usize,
            BitTableOp::Popcnt => BitOp::iter().len(),
        }
    }
}

const BIT_TABLE_POP_CNT_SHIFT: u64 = 208;

fn encode_bit_table<T: FromBn>(op: T, left: T, right: T, result: T) -> T {
    op * T::from_bn(&(BigUint::from(1u64) << 192))
        + left * T::from_bn(&(BigUint::from(1u64) << 128))
        + right * T::from_bn(&(BigUint::from(1u64) << 64))
        + result
}

pub fn encode_bit_table_binary<T: FromBn>(op: T, left: T, right: T, result: T) -> T {
    encode_bit_table(op, left, right, result)
}

pub fn encode_bit_table_popcnt<T: FromBn>(operand: T, result: T) -> T {
    T::from_bn(&(BigUint::one() << BIT_TABLE_POP_CNT_SHIFT))
        + encode_bit_table(
            T::from_bn(&BigUint::from(BitTableOp::Popcnt.index() as u64)),
            operand,
            T::from_bn(&BigUint::zero()),
            result,
        )
}

/// A table to support bit operations('and'/'or'/'xor') and unary operation('popcnt').
#[derive(Clone)]
pub struct BitTableConfig<F: FieldExt> {
    step_sel: Column<Fixed>,
    lookup_sel: Column<Fixed>,
    values: [Column<Advice>; 2],
    _mark: PhantomData<F>,
}
/*
| step_sel  | lookup_sel |  val(col 0) |  val(col 1) |
|    1      |     0      |  encode     |             |
|    0      |     1      |  op         |  op         |
|    0      |     0      |  l_u8_0     |  l_u8_1     |
|    0      |     0      |  r_u8_0     |  r_u8_1     |
|    0      |     0      |  res_u8_0   |  res_u8_1   |
|    0      |     1      |  op         |  op         |
|    0      |     0      |  l_u8_2     |  l_u8_3     |
|    0      |     0      |  r_u8_2     |  r_u8_3     |
|    0      |     0      |  res_u8_2   |  res_u8_3   |
...
|    0      |     1      |  op         |  op         |
|    0      |     0      |  l_u8_6     |  l_u8_7     |
|    0      |     0      |  r_u8_6     |  r_u8_7     |
|    0      |     0      |  res_u8_6   |  res_u8_7   |

|    1      |     0      |  encode    |
|    0      |     1      |  op        |
*/
impl<F: FieldExt> BitTableConfig<F> {
    pub(crate) fn configure(meta: &mut ConstraintSystem<F>, rtable: &RangeTableConfig<F>) -> Self {
        let step_sel = meta.fixed_column();
        let lookup_sel = meta.fixed_column();
        let values = [(); 2].map(|_| meta.advice_column());

        for value in values {
            rtable.configure_in_u8_range(meta, "bit table u8", |meta| {
                (constant_from!(1) - fixed_curr!(meta, step_sel)) * curr!(meta, value)
            });

            rtable.configure_in_u8_bit_table(
                meta,
                "bit table u8 bit table lookup",
                |meta| curr!(meta, value),
                |meta| nextn!(meta, value, 1),
                |meta| nextn!(meta, value, 2),
                |meta| nextn!(meta, value, 3),
                // Constrain bit relation for all steps, enable bit is not necessary.
                |meta| fixed_curr!(meta, lookup_sel),
            );
        }

        meta.create_gate("bit table encode", |meta| {
            macro_rules! compose_u64 {
                ($offset:expr) => {
                    (0..4)
                        .into_iter()
                        .map(|x| {
                            (nextn!(meta, values[0], 1 + x * 4 + $offset)
                                + nextn!(meta, values[1], 1 + x * 4 + $offset)
                                    * constant_from!(1u64 << 8))
                                * constant_from!(1u64 << (16 * x))
                        })
                        .fold(constant_from!(0), |acc, x| acc + x)
                };
            }

            macro_rules! acc_u64 {
                ($offset:expr) => {
                    (0..4)
                        .into_iter()
                        .map(|x| {
                            (nextn!(meta, values[0], 1 + x * 4 + $offset)
                                + nextn!(meta, values[1], 1 + x * 4 + $offset))
                        })
                        .fold(constant_from!(0), |acc, x| acc + x)
                };
            }

            let op = next!(meta, values[0]);
            let left = compose_u64!(1);
            let right = compose_u64!(2);
            let result_compose = compose_u64!(3);
            let result_acc = acc_u64!(3);

            vec![
                (curr!(meta, values[0])
                    - ((constant_from!(1) - curr!(meta, values[1]))
                        * encode_bit_table_binary(
                            op.clone(),
                            left.clone(),
                            right.clone(),
                            result_compose.clone(),
                        ))
                    - (curr!(meta, values[1]) * encode_bit_table_popcnt(left, result_acc)))
                    * fixed_curr!(meta, step_sel),
            ]
        });

        meta.create_gate("op consistent", |meta| {
            vec![
                (nextn!(meta, values[0], 4) - curr!(meta, values[0]))
                    * (fixed_nextn!(meta, step_sel, 4) - constant_from!(1))
                    * fixed_curr!(meta, lookup_sel),
                (curr!(meta, values[0]) - curr!(meta, values[1])) * fixed_curr!(meta, lookup_sel),
            ]
        });

        meta.create_gate("unary selector", |meta| {
            vec![
                fixed_curr!(meta, step_sel)
                    * (curr!(meta, values[1]) * (constant_from!(1) - curr!(meta, values[1]))),
            ]
        });

        Self {
            step_sel,
            lookup_sel,
            values,
            _mark: PhantomData,
        }
    }
}

pub struct BitTableChip<F: FieldExt> {
    config: BitTableConfig<F>,
    max_available_rows: usize,
}

impl<F: FieldExt> BitTableChip<F> {
    pub fn new(config: BitTableConfig<F>) -> Self {
        BitTableChip {
            config,
            max_available_rows: max_bit_table_rows() as usize / STEP_SIZE * STEP_SIZE,
        }
    }
}
