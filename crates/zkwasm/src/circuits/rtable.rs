use super::config::common_range;
use super::config::POW_TABLE_POWER_START;
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

const POW_OP: u64 = 4;

/*
 * | Comment   | Op  | left(u8) | right                       | result   |
 * | --------- | --- | -------- | --------------------------- | -------- |
 * | Bit(And)  | 0   | 0        | 0                           | 0        |
 * | ...       | ... | ...      | ...                         | ...      |
 * | Bit(And)  | 0   | 0xff     | 0xff                        | 0xff     |
 * | Bit(Or)   | 1   | 0        | 0                           | 0        |
 * | ...       | ... | ...      | ...                         | ...      |
 * | Bit(Or)   | 1   | 0xff     | 0xff                        | 0xff     |
 * | Bit(Xor)  | 2   | 0        | 0                           | 0        |
 * | ...       | ... | ...      | ...                         | ...      |
 * | Bit(Xor)  | 2   | 0xff     | 0xff                        | 0        |
 * | Popcnt    | 3   | 0        | /                           | 0        |
 * | ...       | ... | ...      | ...                         | ...      |
 * | Popcnt    | 3   | 0xff     | /                           | 8        |
 * | Power     | 4   | /        | 0                           | 0        |
 * | Power     | 4   | /        | POW_TABLE_POWER_START + 0   | 1 << 0   |
 * | ...       | ... | ...      | ...                         | ...      |
 * | Power     | 4   | /        | POW_TABLE_POWER_START + 127 | 1 << 127 |
 */
#[derive(Clone)]
struct OpTable {
    op: TableColumn,
    left: TableColumn,
    right: TableColumn,
    result: TableColumn,
}

#[derive(Clone)]
pub struct RangeTableConfig<F: FieldExt> {
    // [0 .. common_range())
    common_range_col: TableColumn,
    // [0 .. 65536)
    u16_col: TableColumn,
    // [0 .. 256)
    u8_col: TableColumn,

    op_table: OpTable,

    _mark: PhantomData<F>,
}

pub fn pow_table_power_encode<T: FromBn>(power: T) -> T {
    T::from_bn(&BigUint::from(POW_TABLE_POWER_START)) + power
}

impl<F: FieldExt> RangeTableConfig<F> {
    pub fn configure(meta: &mut ConstraintSystem<F>) -> Self {
        // Shared by u8 lookup and bit table lookup
        let u8_col_multiset = meta.lookup_table_column();

        RangeTableConfig {
            common_range_col: meta.lookup_table_column(),
            u16_col: meta.lookup_table_column(),
            u8_col: u8_col_multiset,
            op_table: OpTable {
                op: meta.lookup_table_column(),
                left: u8_col_multiset,
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

    pub fn configure_in_op_table(
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
                (enable(meta) * op(meta), self.op_table.op),
                (enable(meta) * left(meta), self.op_table.left),
                (enable(meta) * right(meta), self.op_table.right),
                (enable(meta) * result(meta), self.op_table.result),
            ]
        });
    }

    pub fn configure_in_pow_set(
        &self,
        meta: &mut ConstraintSystem<F>,
        key: &'static str,
        exp: impl FnOnce(&mut VirtualCells<'_, F>) -> Expression<F>,
        pow: impl FnOnce(&mut VirtualCells<'_, F>) -> Expression<F>,
        enable: impl Fn(&mut VirtualCells<'_, F>) -> Expression<F>,
    ) {
        self.configure_in_op_table(
            meta,
            key,
            |_| constant_from!(POW_OP),
            |_| constant_from!(0),
            |meta| exp(meta),
            |meta| pow(meta),
            enable,
        );
    }
}

pub struct RangeTableChip<F: FieldExt> {
    config: RangeTableConfig<F>,
}

impl<F: FieldExt> RangeTableChip<F> {
    pub fn new(config: RangeTableConfig<F>) -> Self {
        RangeTableChip { config }
    }

    pub fn init(&self, layouter: &mut impl Layouter<F>, k: u32) -> Result<(), Error> {
        layouter.assign_table(
            || "common range table",
            |mut table| {
                for i in 0..common_range(k) {
                    table.assign_cell(
                        || "range table",
                        self.config.common_range_col,
                        i as usize,
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

        {
            let mut offset = 0;

            layouter.assign_table(
                || "op lookup table",
                |mut table| {
                    for op in BitOp::iter() {
                        for left in 0..1u16 << 8 {
                            for right in 0u16..1 << 8 {
                                table.assign_cell(
                                    || "range table",
                                    self.config.op_table.op,
                                    offset as usize,
                                    || Ok(F::from(op as u64)),
                                )?;

                                table.assign_cell(
                                    || "range table",
                                    self.config.op_table.left,
                                    offset as usize,
                                    || Ok(F::from(left as u64)),
                                )?;

                                table.assign_cell(
                                    || "range table",
                                    self.config.op_table.right,
                                    offset as usize,
                                    || Ok(F::from(right as u64)),
                                )?;

                                table.assign_cell(
                                    || "range table",
                                    self.config.op_table.result,
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
                            self.config.op_table.op,
                            offset as usize,
                            || Ok(F::from(BitTableOp::Popcnt.index() as u64)),
                        )?;

                        table.assign_cell(
                            || "range table",
                            self.config.op_table.left,
                            offset as usize,
                            || Ok(F::from(left as u64)),
                        )?;

                        table.assign_cell(
                            || "range table",
                            self.config.op_table.right,
                            offset as usize,
                            || Ok(F::from(0)),
                        )?;

                        table.assign_cell(
                            || "range table",
                            self.config.op_table.result,
                            offset as usize,
                            || Ok(F::from(left.count_ones() as u64)),
                        )?;

                        offset += 1;
                    }

                    assert_eq!(BitTableOp::Popcnt.index() + 1, POW_OP as usize);

                    {
                        table.assign_cell(
                            || "range table",
                            self.config.op_table.op,
                            offset,
                            || Ok(F::from(POW_OP)),
                        )?;

                        table.assign_cell(
                            || "range table",
                            self.config.op_table.left,
                            offset,
                            || Ok(F::zero()),
                        )?;

                        table.assign_cell(
                            || "range table",
                            self.config.op_table.right,
                            offset,
                            || Ok(F::zero()),
                        )?;

                        table.assign_cell(
                            || "range table",
                            self.config.op_table.result,
                            offset,
                            || Ok(F::zero()),
                        )?;

                        offset += 1;

                        for i in 0..POW_TABLE_POWER_START {
                            table.assign_cell(
                                || "range table",
                                self.config.op_table.op,
                                offset,
                                || Ok(F::from(POW_OP)),
                            )?;

                            table.assign_cell(
                                || "range table",
                                self.config.op_table.left,
                                offset,
                                || Ok(F::zero()),
                            )?;

                            table.assign_cell(
                                || "range table",
                                self.config.op_table.right,
                                offset,
                                || Ok(F::from(POW_TABLE_POWER_START + i)),
                            )?;

                            table.assign_cell(
                                || "range table",
                                self.config.op_table.result,
                                offset,
                                || Ok(bn_to_field::<F>(&(BigUint::from(1u64) << i))),
                            )?;

                            offset += 1;
                        }
                    }

                    Ok(())
                },
            )?;
        }

        Ok(())
    }
}
