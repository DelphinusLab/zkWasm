use super::config::zkwasm_k;
use super::config::POW_TABLE_LIMIT;
use super::utils::bn_to_field;
use crate::circuits::bit_table::BitTableOp;
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
struct U8BitTable {
    op: TableColumn,
    left: TableColumn,
    right: TableColumn,
    result: TableColumn,
}

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

    /*
     * and | or | xor | popcnt,
     * l: u8,
     * r: u8,
     * res: u8
     */
    u8_bit_op_col: U8BitTable,

    _mark: PhantomData<F>,
}

pub fn pow_table_power_encode<T: FromBn>(power: T) -> T {
    T::from_bn(&BigUint::from(POW_TABLE_LIMIT)) + power
}

impl<F: FieldExt> RangeTableConfig<F> {
    pub fn configure(meta: &mut ConstraintSystem<F>) -> Self {
        RangeTableConfig {
            common_range_col: meta.lookup_table_column(),
            u16_col: meta.lookup_table_column(),
            u8_col: meta.lookup_table_column(),
            pow_col: [meta.lookup_table_column(), meta.lookup_table_column()],
            u8_bit_op_col: U8BitTable {
                op: meta.lookup_table_column(),
                left: meta.lookup_table_column(),
                right: meta.lookup_table_column(),
                result: meta.lookup_table_column(),
            },
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
        result: impl FnOnce(&mut VirtualCells<'_, F>) -> Expression<F>,
        enable: impl Fn(&mut VirtualCells<'_, F>) -> Expression<F>,
    ) {
        meta.lookup(key, |meta| {
            vec![
                (enable(meta) * op(meta), self.u8_bit_op_col.op),
                (enable(meta) * left(meta), self.u8_bit_op_col.left),
                (enable(meta) * right(meta), self.u8_bit_op_col.right),
                (enable(meta) * result(meta), self.u8_bit_op_col.result),
            ]
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
                        for left in 0..1u16 << 8 {
                            for right in 0u16..1 << 8 {
                                table.assign_cell(
                                    || "range table",
                                    self.config.u8_bit_op_col.op,
                                    offset as usize,
                                    || Ok(F::from(op as u64)),
                                )?;

                                table.assign_cell(
                                    || "range table",
                                    self.config.u8_bit_op_col.left,
                                    offset as usize,
                                    || Ok(F::from(left as u64)),
                                )?;

                                table.assign_cell(
                                    || "range table",
                                    self.config.u8_bit_op_col.right,
                                    offset as usize,
                                    || Ok(F::from(right as u64)),
                                )?;

                                table.assign_cell(
                                    || "range table",
                                    self.config.u8_bit_op_col.result,
                                    offset as usize,
                                    || Ok(F::from(op.eval(left as u64, right as u64))),
                                )?;

                                offset += 1;
                            }
                        }
                    }

                    for left in 0..1u16 << 8 {
                        table.assign_cell(
                            || "range table",
                            self.config.u8_bit_op_col.op,
                            offset as usize,
                            || Ok(F::from(BitTableOp::Popcnt.index() as u64)),
                        )?;

                        table.assign_cell(
                            || "range table",
                            self.config.u8_bit_op_col.left,
                            offset as usize,
                            || Ok(F::from(left as u64)),
                        )?;

                        table.assign_cell(
                            || "range table",
                            self.config.u8_bit_op_col.right,
                            offset as usize,
                            || Ok(F::from(0)),
                        )?;

                        table.assign_cell(
                            || "range table",
                            self.config.u8_bit_op_col.result,
                            offset as usize,
                            || Ok(F::from(left.count_ones() as u64)),
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
