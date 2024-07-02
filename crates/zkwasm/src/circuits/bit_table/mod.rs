use std::marker::PhantomData;

use halo2_proofs::arithmetic::FieldExt;
use halo2_proofs::plonk::Advice;
use halo2_proofs::plonk::Column;
use halo2_proofs::plonk::ConstraintSystem;
use halo2_proofs::plonk::Fixed;
use specs::itable::BitOp;
use strum::IntoEnumIterator;

use crate::constant_from;
use crate::curr;
use crate::fixed_curr;
use crate::nextn;
use crate::prev;

use self::assign::BitTableAssign;

use super::rtable::RangeTableConfig;

mod assign;
mod configure;

pub(crate) trait BitTableTrait {
    fn filter_bit_table_entries(&self) -> Vec<BitTableAssign>;
}

#[derive(Clone, Copy, PartialEq)]
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

/// A table to support bit operations('and'/'or'/'xor') and unary operation('popcnt').
#[derive(Clone)]
pub struct BitTableConfig<F: FieldExt> {
    block_sel: Column<Fixed>,
    u32_sel: Column<Fixed>,
    lookup_sel: Column<Fixed>,

    op: Column<Advice>,
    helper: Column<Advice>,
    left: Column<Advice>,
    right: Column<Advice>,
    result: Column<Advice>,

    _mark: PhantomData<F>,
}

pub(self) const STEP_SIZE: usize = 11;
pub(self) const BLOCK_SEL_OFFSET: usize = 1;
pub(self) const U32_OFFSET: [usize; 2] = [1, 6];
pub(self) const U8_OFFSET: [usize; 8] = [2, 3, 4, 5, 7, 8, 9, 10];

/*
 * Columns:
 * --------------------------------------------------------------------------------
 * block: enable etable lookup. put the bit on the second line to minimize
 *        the number of rotations as much as possible.
 * u32_sel: selector to accumulate(if popcnt) or compose(if bit op) 4 * u8 into u32
 * lookup_sel: lookup (op, l_u8, r_u8, res_u8) in rtable
 * op: and: 0, or: 1, xor: 2, popcnt: 3
 * val: u64 value, split u64 value into u32, split u32 value into u8
 *
 * |   block   | u32_sel | lookup_sel |  op  |  helper    |    val_l    |    val_r    |   val_res   |
 * +-----------+-------- |------------+------+------------|-------------+-------------+-------------+
*  |     0     |    0    |     0      |   x  |            |  l_u64      |  r_u64      | res_u64     |
 * |     1     |    1    |     0      |   x  | is_popcnt  |  l_u32[0]   |  r_u32[0]   | res_u32[0]  |
 * |     0     |    0    |     1      |   x  |            |   l_u8[0]   |   r_u8[0]   |  res_u8[0]  |
 * |     0     |    0    |     1      |   x  |            |   l_u8[1]   |   r_u8[1]   |  res_u8[1]  |
 * |     0     |    0    |     1      |   x  |            |   l_u8[2]   |   r_u8[2]   |  res_u8[2]  |
 * |     0     |    0    |     1      |   x  |            |   l_u8[3]   |   r_u8[3]   |  res_u8[3]  |
 * |     0     |    1    |     0      |   x  | is_popcnt  |  l_u32[1]   |  r_u32[1]   | res_u32[1]  |
 * |     0     |    0    |     1      |   x  |            |   l_u8[0]   |   r_u8[0]   |  res_u8[0]  |
 * |     0     |    0    |     1      |   x  |            |   l_u8[1]   |   r_u8[1]   |  res_u8[1]  |
 * |     0     |    0    |     1      |   x  |            |   l_u8[2]   |   r_u8[2]   |  res_u8[2]  |
 * |     0     |    0    |     1      |   x  |            |   l_u8[3]   |   r_u8[3]   |  res_u8[3]  |
 * +-----------+---------|------------+------+------------|-------------+-------------+-------------+
 */
impl<F: FieldExt> BitTableConfig<F> {
    /*
     * Constraints:
     * ------------------------------------------------------------------------------------------------
     * 1. * 'op' should be consistent within a block.
     *    * is_popcnt cell is set when op is Popcnt
     *    * is_popcnt is bit
     * 2. * l/r/res_u32 = l/r/res_u8[0] + l/r/res_u8[1] << 8 + l/r/res_u8[2] <<16 + l/r/res_u8[3] << 24
     *      if is unary.
     *    * l/r/res_u32 = l/r/res_u8[0] + l/r/res_u8[1] + l/r/res_u8[2]  + l/r/res_u8[3]
     *      if is unary.
     * 3. * l/r/res_u64 = l/r/res_u32[0] + l/r/res_u32[1] << 32 if is unary.
     *    * l/r/res_u64 = l/r/res_u32[0] + l/r/res_u32[1] if is unary.
     *
     * Lookup:
     * 1. lookup (op, l_u8, r_u8, res_u8) in rtable if lookup_sel is enabled.
     * 2. etable lookups (op, l_u64, r_u64, res_u64) in this table's entries.
     */
    pub(crate) fn configure(meta: &mut ConstraintSystem<F>, rtable: &RangeTableConfig<F>) -> Self {
        let block_sel = meta.fixed_column();
        let u32_sel = meta.fixed_column();
        let lookup_sel = meta.fixed_column();
        let op = meta.advice_column();
        let helper = meta.advice_column();
        let left = meta.advice_column();
        let right = meta.advice_column();
        let result = meta.advice_column();

        rtable.configure_in_op_table(
            meta,
            "bit table lookup in rtable",
            |meta| curr!(meta, op),
            |meta| curr!(meta, left),
            |meta| curr!(meta, right),
            |meta| curr!(meta, result),
            |meta| fixed_curr!(meta, lookup_sel),
        );

        meta.create_gate("bit table: 1. op consistent", |meta| {
            vec![
                (fixed_curr!(meta, u32_sel) + fixed_curr!(meta, lookup_sel))
                    * (prev!(meta, op) - curr!(meta, op)),
                fixed_curr!(meta, u32_sel)
                    * curr!(meta, helper)
                    * (curr!(meta, op) - constant_from!(BitTableOp::Popcnt.index())),
                fixed_curr!(meta, u32_sel)
                    * (curr!(meta, helper) - constant_from!(1))
                    * curr!(meta, op)  // - constant_from!(BitOp::And)): 0
                    * (curr!(meta, op) - constant_from!(BitOp::Or))
                    * (curr!(meta, op) - constant_from!(BitOp::Xor)),
                // is_popcnt cell is bit
                fixed_curr!(meta, u32_sel)
                    * curr!(meta, helper)
                    * (curr!(meta, helper) - constant_from!(1)),
            ]
        });

        meta.create_gate("bit table: 2. acc u32", |meta| {
            let is_popcnt = curr!(meta, helper);
            let is_bit = constant_from!(1) - is_popcnt.clone();

            // For bit operator
            macro_rules! compose_u32_helper {
                ($col:expr) => {
                    (0..4)
                        .into_iter()
                        .map(|x| {
                            if x == 0 {
                                nextn!(meta, $col, 1)
                            } else {
                                (nextn!(meta, $col, x + 1)) * constant_from!(1u64 << (8 * x))
                            }
                        })
                        .reduce(|acc, x| acc + x)
                        .unwrap()
                };
            }

            // For popcnt operator
            macro_rules! acc_u32_helper {
                ($col:expr) => {
                    (0..4)
                        .into_iter()
                        .map(|x| (nextn!(meta, $col, 1 + x)))
                        .reduce(|acc, x| acc + x)
                        .unwrap()
                };
            }

            macro_rules! compose_u32 {
                ($col:ident) => {
                    fixed_curr!(meta, u32_sel) * (compose_u32_helper!($col) - curr!(meta, $col))
                };
            }

            macro_rules! compose_u32_if_bit {
                ($col:ident) => {
                    compose_u32!($col) * is_bit.clone()
                };
            }

            macro_rules! acc_u32_if_popcnt {
                ($col:ident) => {
                    fixed_curr!(meta, u32_sel)
                        * (acc_u32_helper!($col) - curr!(meta, $col))
                        * is_popcnt
                };
            }

            vec![
                compose_u32!(left),
                compose_u32!(right),
                compose_u32_if_bit!(result),
                acc_u32_if_popcnt!(result),
            ]
        });

        meta.create_gate("bit table: 3. acc u64", |meta| {
            let is_popcnt = curr!(meta, helper);
            let is_bit = constant_from!(1) - is_popcnt.clone();

            macro_rules! compose_u64 {
                ($col: expr) => {
                    fixed_curr!(meta, block_sel)
                        * (prev!(meta, $col)
                            - curr!(meta, $col)
                            - nextn!(meta, $col, 5) * constant_from!(1u64 << 32))
                };
            }

            macro_rules! compose_u64_if_bit {
                ($col: expr) => {
                    compose_u64!($col) * is_bit.clone()
                };
            }

            macro_rules! acc_u64_if_popcnt {
                ($col: expr) => {
                    fixed_curr!(meta, block_sel)
                        * is_popcnt
                        * (prev!(meta, $col) - curr!(meta, $col) - nextn!(meta, $col, 5))
                };
            }

            vec![
                compose_u64!(left),
                compose_u64!(right),
                compose_u64_if_bit!(result),
                acc_u64_if_popcnt!(result),
            ]
        });

        Self {
            block_sel,
            u32_sel,
            lookup_sel,
            op,
            helper,
            left,
            right,
            result,
            _mark: PhantomData,
        }
    }
}

pub struct BitTableChip<F: FieldExt> {
    config: BitTableConfig<F>,
    max_available_rows: usize,
}

impl<F: FieldExt> BitTableChip<F> {
    pub fn new(config: BitTableConfig<F>, max_available_rows: usize) -> Self {
        BitTableChip {
            config,
            max_available_rows: max_available_rows / STEP_SIZE * STEP_SIZE,
        }
    }
}
