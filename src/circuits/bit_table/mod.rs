use std::marker::PhantomData;

use halo2_proofs::{
    arithmetic::FieldExt,
    plonk::{Advice, Column, ConstraintSystem, Fixed},
};
use num_bigint::BigUint;
use specs::encode::FromBn;

use crate::{
    constant_from, constant_from_bn, curr, fixed_curr, fixed_next, fixed_nextn, next, nextn, prev,
};

use super::{
    config::max_bit_table_rows,
    rtable::{encode_u8_bit_entry, RangeTableConfig},
    utils::bn_to_field,
};

mod assign;
mod configure;

const STEP_SIZE: usize = 33;

pub fn encode_bit_table<T: FromBn>(op: T, left: T, right: T, result: T) -> T {
    op * T::from_bn(&(BigUint::from(1u64) << 192))
        + left * T::from_bn(&(BigUint::from(1u64) << 128))
        + right * T::from_bn(&(BigUint::from(1u64) << 64))
        + result
}

#[derive(Clone)]
pub struct BitTableConfig<F: FieldExt> {
    step_sel: Column<Fixed>,
    lookup_sel: Column<Fixed>,
    value: Column<Advice>,
    _mark: PhantomData<F>,
}
/*
| step_sel  | lookup_sel |   val      |
|    1      |     0      |  encode    |
|    0      |     1      |  op        |
|    0      |     0      |  l_u8_0    |
|    0      |     0      |  r_u8_0    |
|    0      |     0      |  res_u8_0  |
|    0      |     1      |  op        |
|    0      |     0      |  l_u8_1    |
|    0      |     0      |  r_u8_1    |
|    0      |     0      |  res_u8_1  |
...
|    0      |     1      |  op        |
|    0      |     0      |  l_u8_7    |
|    0      |     0      |  r_u8_7    |
|    0      |     0      |  res_u8_7  |

|    1      |     0      |  encode    |
|    0      |     1      |  op        |
*/
impl<F: FieldExt> BitTableConfig<F> {
    pub(crate) fn configure(meta: &mut ConstraintSystem<F>, rtable: &RangeTableConfig<F>) -> Self {
        let step_sel = meta.fixed_column();
        let lookup_sel = meta.fixed_column();
        let value = meta.advice_column();

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
            |meta| fixed_curr!(meta, lookup_sel),
        );

        meta.create_gate("bit table encode", |meta| {
            macro_rules! compose_u64 {
                ($offset:expr) => {
                    (0..8)
                        .into_iter()
                        .map(|x| {
                            nextn!(meta, value, 1 + x * 4 + $offset)
                                * constant_from!(1u64 << (8 * x))
                        })
                        .fold(constant_from!(0), |acc, x| acc + x)
                };
            }

            let op = next!(meta, value);
            let left = compose_u64!(1);
            let right = compose_u64!(2);
            let result = compose_u64!(3);

            vec![
                (curr!(meta, value) - encode_bit_table(op, left, right, result))
                    * fixed_curr!(meta, step_sel),
            ]
        });

        meta.create_gate("op consistent", |meta| {
            vec![
                (nextn!(meta, value, 4) - curr!(meta, value))
                    * (fixed_nextn!(meta, step_sel, 4) - constant_from!(1))
                    * fixed_curr!(meta, lookup_sel),
            ]
        });

        Self {
            step_sel,
            lookup_sel,
            value,
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
