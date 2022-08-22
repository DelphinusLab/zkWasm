use super::config::K;
use super::utils::bn_to_field;
use crate::constant_from;
use halo2_proofs::arithmetic::FieldExt;
use halo2_proofs::circuit::Layouter;
use halo2_proofs::plonk::ConstraintSystem;
use halo2_proofs::plonk::Error;
use halo2_proofs::plonk::Expression;
use halo2_proofs::plonk::TableColumn;
use halo2_proofs::plonk::VirtualCells;
use num_bigint::BigUint;
use std::marker::PhantomData;
use strum::IntoEnumIterator;
use strum_macros::EnumIter;

#[derive(Clone)]
pub struct RangeTableConfig<F: FieldExt> {
    // [0 .. 65536)
    u16_col: TableColumn,
    // [0 .. 256)
    u8_col: TableColumn,
    // [0 .. 16)
    u4_col: TableColumn,
    // {(left, right, res, op) | op(left, right) = res}, encoded by concat(left, right, res) << op
    u4_bop_calc_col: TableColumn,
    // {0, 1, 1 << 12, 1 << 24 ...}
    u4_bop_col: TableColumn,
    // {1 | 0, 2 | 1, 4 | 2, ...}
    pow_col: TableColumn,

    _mark: PhantomData<F>,
}

impl<F: FieldExt> RangeTableConfig<F> {
    pub fn configure(cols: [TableColumn; 6]) -> Self {
        RangeTableConfig {
            u16_col: cols[0],
            u8_col: cols[1],
            u4_col: cols[2],
            u4_bop_calc_col: cols[3],
            u4_bop_col: cols[4],
            pow_col: cols[5],
            _mark: PhantomData,
        }
    }

    pub fn configure_in_common_range(
        &self,
        meta: &mut ConstraintSystem<F>,
        key: &'static str,
        expr: impl FnOnce(&mut VirtualCells<'_, F>) -> Expression<F>,
    ) {
        meta.lookup(key, |meta| vec![(expr(meta), self.u16_col)]);
    }

    pub fn configure_in_u16_range(
        &self,
        meta: &mut ConstraintSystem<F>,
        key: &'static str,
        expr: impl FnOnce(&mut VirtualCells<'_, F>) -> Expression<F>,
    ) {
        meta.lookup(key, |meta| vec![(expr(meta), self.u16_col)]);
    }

    pub fn configure_in_u8_range(
        &self,
        meta: &mut ConstraintSystem<F>,
        key: &'static str,
        expr: impl FnOnce(&mut VirtualCells<'_, F>) -> Expression<F>,
    ) {
        meta.lookup(key, |meta| vec![(expr(meta), self.u8_col)]);
    }

    pub fn configure_in_u4_range(
        &self,
        meta: &mut ConstraintSystem<F>,
        key: &'static str,
        expr: impl FnOnce(&mut VirtualCells<'_, F>) -> Expression<F>,
    ) {
        meta.lookup(key, |meta| vec![(expr(meta), self.u4_col)]);
    }

    pub fn configure_in_u4_bop_set(
        &self,
        meta: &mut ConstraintSystem<F>,
        key: &'static str,
        expr: impl FnOnce(&mut VirtualCells<'_, F>) -> Expression<F>,
    ) {
        meta.lookup(key, |meta| vec![(expr(meta), self.u4_bop_col)]);
    }

    pub fn configure_in_u4_bop_calc_set(
        &self,
        meta: &mut ConstraintSystem<F>,
        key: &'static str,
        expr: impl FnOnce(
            &mut VirtualCells<'_, F>,
        ) -> (Expression<F>, Expression<F>, Expression<F>, Expression<F>),
    ) {
        meta.lookup(key, |meta| {
            let (l, r, res, op) = expr(meta);
            vec![(
                (l * constant_from!(1u64 << 8) + r * constant_from!(1u64 << 4) + res) * op,
                self.u4_bop_calc_col,
            )]
        });
    }

    pub fn configure_in_pow_set(
        &self,
        meta: &mut ConstraintSystem<F>,
        key: &'static str,
        expr: impl FnOnce(&mut VirtualCells<'_, F>) -> Expression<F>,
    ) {
        meta.lookup(key, |meta| vec![(expr(meta), self.pow_col)]);
    }

    pub fn configure_in_vtype_byte_range(
        &self,
        meta: &mut ConstraintSystem<F>,
        key: &'static str,
        expr: impl FnOnce(&mut VirtualCells<'_, F>) -> (Expression<F>, Expression<F>, Expression<F>),
        enable: impl FnOnce(&mut VirtualCells<'_, F>) -> Expression<F>,
    ) {
        todo!()
    }
}

pub struct RangeTableChip<F: FieldExt> {
    config: RangeTableConfig<F>,
    _phantom: PhantomData<F>,
}

#[derive(Clone, EnumIter, Copy)]
pub enum BinOp {
    And = 0,
    Or,
    Xor,
    Not,
    Lt,
    Gt,
}

impl BinOp {
    fn left_range(&self) -> usize {
        1 << 4
    }

    fn right_range(&self) -> usize {
        match self {
            BinOp::Not => 1,
            _ => 1 << 4,
        }
    }

    fn calc(&self, left: usize, right: usize) -> usize {
        match self {
            BinOp::And => left & right,
            BinOp::Or => left | right,
            BinOp::Xor => left ^ right,
            BinOp::Not => (!left) & 0xf,
            BinOp::Lt => {
                if left < right {
                    1
                } else {
                    0
                }
            }
            BinOp::Gt => {
                if left > right {
                    1
                } else {
                    0
                }
            }
        }
    }
}

pub fn pow_table_encode<F: FieldExt>(
    modulus: Expression<F>,
    power: Expression<F>,
) -> Expression<F> {
    modulus * constant_from!(1u64 << 16) + power
}

impl<F: FieldExt> RangeTableChip<F> {
    pub fn new(config: RangeTableConfig<F>) -> Self {
        RangeTableChip {
            config,
            _phantom: PhantomData,
        }
    }

    pub fn init(&self, layouter: &mut impl Layouter<F>) -> Result<(), Error> {
        layouter.assign_table(
            || "common range table",
            |mut table| {
                for i in 0..(1 << (K - 1)) {
                    table.assign_cell(
                        || "range table",
                        self.config.u16_col,
                        i,
                        || Ok(F::from(i as u64)),
                    )?;
                }
                Ok(())
            },
        )?;

        layouter.assign_table(
            || "u8 range table",
            |mut table| {
                for i in 0..(1 << 8) {
                    table.assign_cell(
                        || "range table",
                        self.config.u8_col,
                        i,
                        || Ok(F::from(i as u64)),
                    )?;
                }
                Ok(())
            },
        )?;

        layouter.assign_table(
            || "u4 range table",
            |mut table| {
                for i in 0..(1 << 4) {
                    table.assign_cell(
                        || "range table",
                        self.config.u4_col,
                        i,
                        || Ok(F::from(i as u64)),
                    )?;
                }
                Ok(())
            },
        )?;

        layouter.assign_table(
            || "u4 bop set table",
            |mut table| {
                table.assign_cell(
                    || "range table",
                    self.config.u4_bop_col,
                    0,
                    || Ok(F::from(0 as u64)),
                )?;
                let mut offset = 1;
                for i in BinOp::iter() {
                    table.assign_cell(
                        || "range table",
                        self.config.u4_bop_col,
                        offset,
                        || {
                            Ok(bn_to_field::<F>(
                                &(BigUint::from(1u64) << (12 * i as usize)),
                            ))
                        },
                    )?;
                    offset += 1;
                }
                Ok(())
            },
        )?;

        layouter.assign_table(
            || "u4 bop calc table",
            |mut table| {
                table.assign_cell(
                    || "range table",
                    self.config.u4_bop_calc_col,
                    0,
                    || Ok(F::from(0 as u64)),
                )?;
                let mut offset = 1;
                for i in BinOp::iter() {
                    for l in 0..i.left_range() {
                        for r in 0..i.right_range() {
                            let res = i.calc(l, r);
                            table.assign_cell(
                                || "range table",
                                self.config.u4_bop_calc_col,
                                offset as usize,
                                || {
                                    Ok(F::from((l * 256 + r * 16 + res) as u64)
                                        * bn_to_field::<F>(
                                            &(BigUint::from(1u64) << (i as usize * 12)),
                                        ))
                                },
                            )?;
                        }
                    }
                    offset += 1;
                }
                Ok(())
            },
        )?;

        layouter.assign_table(
            || "pow table",
            |mut table| {
                table.assign_cell(
                    || "range table",
                    self.config.pow_col,
                    0,
                    || Ok(F::from(0 as u64)),
                )?;
                let mut offset = 1;
                for i in 0..128usize {
                    table.assign_cell(
                        || "range table",
                        self.config.pow_col,
                        offset as usize,
                        || {
                            Ok(bn_to_field::<F>(&(BigUint::from(1u64) << (i + 16)))
                                + F::from(i as u64))
                        },
                    )?;
                    offset += 1;
                }
                Ok(())
            },
        )?;

        Ok(())
    }
}
