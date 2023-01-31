use super::config::zkwasm_k;
use super::config::POW_TABLE_LIMIT;
use super::utils::bn_to_field;
use crate::constant_from;
use crate::traits::circuits::bit_range_table::BitRangeTable;
use halo2_proofs::arithmetic::FieldExt;
use halo2_proofs::circuit::Layouter;
use halo2_proofs::plonk::ConstraintSystem;
use halo2_proofs::plonk::Error;
use halo2_proofs::plonk::Expression;
use halo2_proofs::plonk::TableColumn;
use halo2_proofs::plonk::VirtualCells;
use num_bigint::BigUint;
use specs::itable::BitOp;
use std::marker::PhantomData;
use strum::IntoEnumIterator;

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
    // {0 | 1 | 0b1000000000000000, 0 | 2 | 0b110000000000000 ...}
    offset_len_bits_col: TableColumn,

    _mark: PhantomData<F>,
}

impl<F: FieldExt> RangeTableConfig<F> {
    pub fn configure(cols: [TableColumn; 7]) -> Self {
        RangeTableConfig {
            u16_col: cols[0],
            u8_col: cols[1],
            u4_col: cols[2],
            u4_bop_calc_col: cols[3],
            u4_bop_col: cols[4],
            pow_col: cols[5],
            offset_len_bits_col: cols[6],
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

    pub fn configure_in_offset_len_bits_set(
        &self,
        meta: &mut ConstraintSystem<F>,
        key: &'static str,
        expr: impl FnOnce(&mut VirtualCells<'_, F>) -> Expression<F>,
    ) {
        meta.lookup(key, |meta| vec![(expr(meta), self.offset_len_bits_col)]);
    }
}

pub struct RangeTableChip<F: FieldExt> {
    config: RangeTableConfig<F>,
}

pub fn pow_table_encode<F: FieldExt>(
    modulus: Expression<F>,
    power: Expression<F>,
) -> Expression<F> {
    modulus * constant_from!(1u64 << 16) + power
}

pub fn bits_of_offset_len(offset: u64, len: u64) -> u64 {
    let bits = (1 << len) - 1;
    bits << offset
}

pub fn offset_len_bits_encode(offset: u64, len: u64) -> u64 {
    assert!(offset < 16);
    assert!(len == 1 || len == 2 || len == 4 || len == 8);
    (offset << 20) + (len << 16) + bits_of_offset_len(offset, len)
}

pub fn offset_len_bits_encode_expr<F: FieldExt>(
    offset: Expression<F>,
    len: Expression<F>,
    bits: Expression<F>,
) -> Expression<F> {
    offset * constant_from!(1u64 << 20) + len * constant_from!(1u64 << 16) + bits
}

impl<F: FieldExt> RangeTableChip<F> {
    pub fn new(config: RangeTableConfig<F>) -> Self {
        RangeTableChip { config }
    }

    pub fn init(&self, layouter: &mut impl Layouter<F>) -> Result<(), Error> {
        layouter.assign_table(
            || "common range table",
            |mut table| {
                for i in 0..(1 << (zkwasm_k() - 1)) {
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
                for i in BitOp::iter() {
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
                for i in BitOp::iter() {
                    for l in 0..1 << 4 {
                        for r in 0..1 << 4 {
                            let res = i.eval(l, r);
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
                            offset += 1;
                        }
                    }
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
                for i in 0..POW_TABLE_LIMIT {
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

        layouter.assign_table(
            || "offset len bits table",
            |mut table| {
                table.assign_cell(
                    || "range table",
                    self.config.offset_len_bits_col,
                    0,
                    || Ok(F::from(0 as u64)),
                )?;
                let mut offset = 1;
                for i in 0..8 {
                    for j in vec![1, 2, 4, 8] {
                        table.assign_cell(
                            || "range table",
                            self.config.offset_len_bits_col,
                            offset as usize,
                            || Ok(F::from(offset_len_bits_encode(i, j))),
                        )?;
                        offset += 1;
                    }
                }
                Ok(())
            },
        )?;

        Ok(())
    }
}

impl<F: FieldExt> BitRangeTable<F> for RangeTableConfig<F> {
    fn configure_in_u4_range(
        &self,
        meta: &mut ConstraintSystem<F>,
        key: &'static str,
        expr: impl FnOnce(&mut VirtualCells<'_, F>) -> Expression<F>,
    ) {
        self.configure_in_u4_range(meta, key, expr);
    }

    fn configure_in_u8_range(
        &self,
        meta: &mut ConstraintSystem<F>,
        key: &'static str,
        expr: impl FnOnce(&mut VirtualCells<'_, F>) -> Expression<F>,
    ) {
        self.configure_in_u8_range(meta, key, expr);
    }
}
