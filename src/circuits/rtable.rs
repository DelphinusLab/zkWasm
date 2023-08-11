use super::config::zkwasm_k;
use super::config::POW_TABLE_LIMIT;
use super::utils::bn_to_field;
use crate::circuits::bit_table::BitTableOp;
use crate::constant_from;
use halo2_proofs::arithmetic::FieldExt;
use halo2_proofs::circuit::Layouter;
use halo2_proofs::plonk::ConstraintSystem;
use halo2_proofs::plonk::Error;
use halo2_proofs::plonk::Expression;
use halo2_proofs::plonk::TableColumn;
use halo2_proofs::plonk::VirtualCells;
use num_bigint::BigUint;
use specs::encode::FromBn;
use specs::itable::BitOp;
use std::marker::PhantomData;
use strum::IntoEnumIterator;

#[derive(Clone)]
pub struct RangeTableConfig<F: FieldExt> {
    // [0 .. 1 << zkwasm_k() - 1)
    common_range_col: TableColumn,
    // [0 .. 65536)
    u16_col: TableColumn,
    // [0 .. 256)
    u8_col: TableColumn,

    /*
    {
        0 | 0,
        1 | PREFIX + 0,
        2 | PREFIX + 1,
        4 | PREFIX + 2,
        ...
    }
    */
    pow_col: [TableColumn; 2],

    // (and | or | xor | popcnt) << 24
    //        l_u8               << 16
    //        r_u8               <<  8
    //      res_u8
    u8_bit_op_col: TableColumn,

    _mark: PhantomData<F>,
}

pub fn pow_table_power_encode<T: FromBn>(power: T) -> T {
    T::from_bn(&BigUint::from(POW_TABLE_LIMIT)) + power
}

pub(crate) fn encode_u8_bit_lookup(op: BitOp, left: u8, right: u8) -> u64 {
    let res = op.eval(left as u64, right as u64);
    ((op as u64) << 24) + ((left as u64) << 16) + ((right as u64) << 8) + res
}

pub(crate) fn encode_u8_popcnt_lookup(value: u8) -> u64 {
    ((BitTableOp::Popcnt.index() as u64) << 24)
        + ((value as u64) << 16)
        + (value.count_ones() as u64)
}

impl<F: FieldExt> RangeTableConfig<F> {
    pub fn configure(mut cols: impl Iterator<Item = TableColumn>) -> Self {
        RangeTableConfig {
            common_range_col: cols.next().unwrap(),
            u16_col: cols.next().unwrap(),
            u8_col: cols.next().unwrap(),
            pow_col: [cols.next().unwrap(), cols.next().unwrap()],
            u8_bit_op_col: cols.next().unwrap(),
            _mark: PhantomData,
        }
    }

    pub fn configure_in_common_range(
        &self,
        meta: &mut ConstraintSystem<F>,
        key: &'static str,
        expr: impl FnOnce(&mut VirtualCells<'_, F>) -> Expression<F>,
    ) {
        meta.lookup(key, |meta| vec![(expr(meta), self.common_range_col)]);
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

    pub fn configure_in_u8_bit_table(
        &self,
        meta: &mut ConstraintSystem<F>,
        key: &'static str,
        op: impl FnOnce(&mut VirtualCells<'_, F>) -> Expression<F>,
        left: impl FnOnce(&mut VirtualCells<'_, F>) -> Expression<F>,
        right: impl FnOnce(&mut VirtualCells<'_, F>) -> Expression<F>,
        res: impl FnOnce(&mut VirtualCells<'_, F>) -> Expression<F>,
        enable: impl FnOnce(&mut VirtualCells<'_, F>) -> Expression<F>,
    ) {
        meta.lookup(key, |meta| {
            vec![(
                enable(meta)
                    * (op(meta) * constant_from!(1 << 24)
                        + left(meta) * constant_from!(1 << 16)
                        + right(meta) * constant_from!(1 << 8)
                        + res(meta)),
                self.u8_bit_op_col,
            )]
        });
    }

    pub fn configure_in_pow_set(
        &self,
        meta: &mut ConstraintSystem<F>,
        key: &'static str,
        expr: impl FnOnce(&mut VirtualCells<'_, F>) -> [Expression<F>; 2],
    ) {
        meta.lookup(key, |meta| {
            let [e0, e1] = expr(meta);
            vec![(e0, self.pow_col[0]), (e1, self.pow_col[1])]
        });
    }
}

pub struct RangeTableChip<F: FieldExt> {
    config: RangeTableConfig<F>,
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
                        self.config.common_range_col,
                        i,
                        || Ok(F::from(i as u64)),
                    )?;
                }
                Ok(())
            },
        )?;

        layouter.assign_table(
            || "u16 range table",
            |mut table| {
                for i in 0..(1 << 16) {
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
            || "pow table",
            |mut table| {
                table.assign_cell(
                    || "range table",
                    self.config.pow_col[0],
                    0,
                    || Ok(F::from(0 as u64)),
                )?;
                table.assign_cell(
                    || "range table",
                    self.config.pow_col[1],
                    0,
                    || Ok(F::from(0 as u64)),
                )?;
                let mut offset = 1;
                for i in 0..POW_TABLE_LIMIT {
                    table.assign_cell(
                        || "range table",
                        self.config.pow_col[0],
                        offset as usize,
                        || Ok(bn_to_field::<F>(&(BigUint::from(1u64) << i))),
                    )?;
                    table.assign_cell(
                        || "range table",
                        self.config.pow_col[1],
                        offset as usize,
                        || Ok(F::from(POW_TABLE_LIMIT + i)),
                    )?;
                    offset += 1;
                }
                Ok(())
            },
        )?;

        {
            let mut offset = 0;

            layouter.assign_table(
                || "u8 bit table",
                |mut table| {
                    for op in BitOp::iter() {
                        for l in 0..1u16 << 8 {
                            for r in 0u16..1 << 8 {
                                table.assign_cell(
                                    || "range table",
                                    self.config.u8_bit_op_col,
                                    offset as usize,
                                    || Ok(F::from(encode_u8_bit_lookup(op, l as u8, r as u8))),
                                )?;
                                offset += 1;
                            }
                        }
                    }

                    for value in 0..1u16 << 8 {
                        table.assign_cell(
                            || "range table",
                            self.config.u8_bit_op_col,
                            offset as usize,
                            || Ok(F::from(encode_u8_popcnt_lookup(value as u8))),
                        )?;
                        offset += 1;
                    }
                    Ok(())
                },
            )?;
        }

        Ok(())
    }
}
