use crate::constant_from;
use crate::constant_from_bn;
use halo2_proofs::arithmetic::FieldExt;
use halo2_proofs::circuit::Layouter;
use halo2_proofs::plonk::ConstraintSystem;
use halo2_proofs::plonk::Error;
use halo2_proofs::plonk::Expression;
use halo2_proofs::plonk::TableColumn;
use halo2_proofs::plonk::VirtualCells;
use num_bigint::BigUint;
use specs::itable::BitOp;
use specs::mtable::VarType;
use std::marker::PhantomData;
use strum::IntoEnumIterator;

use super::utils::bn_to_field;

#[derive(Clone)]
pub struct RangeTableConfig<F: FieldExt> {
    // [0 .. 65536)
    u16_col: TableColumn,
    // [0 .. 256)
    u8_col: TableColumn,
    // [0 .. 16)
    u4_col: TableColumn,
    // compose_of(byte_pos_of_8byte, var_type, byte) to avoid overflow, 3 + 3 + 8 = 14 bits in total
    vtype_byte_col: TableColumn,
    // op | left | right | res
    bitop_col: TableColumn,
    // vartype | offset | pos | byte | value
    byte_shift_res_col: TableColumn,
    // byte shift sets
    byte_shift_validation_col: TableColumn,
    // vartype | offset | pos | changes
    byte_offset_unchanged_validation_col: TableColumn,
    // byte | byte
    byte_lt_col: TableColumn,
    // value | shift
    pow_2_col: TableColumn,
    _mark: PhantomData<F>,
}

impl<F: FieldExt> RangeTableConfig<F> {
    pub fn configure(cols: [TableColumn; 10]) -> Self {
        RangeTableConfig {
            u16_col: cols[0],
            u8_col: cols[1],
            u4_col: cols[2],
            vtype_byte_col: cols[3],
            bitop_col: cols[4],
            byte_shift_res_col: cols[5],
            byte_shift_validation_col: cols[6],
            byte_offset_unchanged_validation_col: cols[7],
            byte_lt_col: cols[8],
            pow_2_col: cols[9],
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

    pub fn configure_in_bitop(
        &self,
        meta: &mut ConstraintSystem<F>,
        key: &'static str,
        enable: impl FnOnce(&mut VirtualCells<'_, F>) -> Expression<F>,
        op: impl FnOnce(&mut VirtualCells<'_, F>) -> Expression<F>,
        left: impl FnOnce(&mut VirtualCells<'_, F>) -> Expression<F>,
        right: impl FnOnce(&mut VirtualCells<'_, F>) -> Expression<F>,
        res: impl FnOnce(&mut VirtualCells<'_, F>) -> Expression<F>,
    ) {
        meta.lookup(key, |meta| {
            vec![(
                enable(meta)
                    * (op(meta) * constant_from!(1 << 12)
                        + left(meta) * constant_from!(1 << 8)
                        + right(meta) * constant_from!(1 << 4)
                        + res(meta)),
                self.u16_col, // TODO: check
            )]
        });
    }

    pub fn configure_in_vtype_byte_range(
        &self,
        meta: &mut ConstraintSystem<F>,
        key: &'static str,
        pos_vtype_byte: impl FnOnce(
            &mut VirtualCells<'_, F>,
        ) -> (Expression<F>, Expression<F>, Expression<F>),
        enable: impl FnOnce(&mut VirtualCells<'_, F>) -> Expression<F>,
    ) {
        meta.lookup(key, |meta| {
            let (pos, vtype, byte) = pos_vtype_byte(meta);

            vec![(
                (pos * constant_from!(1 << 12) + vtype * constant_from!(1 << 8) + byte)
                    * enable(meta),
                self.vtype_byte_col,
            )]
        });
    }

    pub fn configure_in_unchangable_range(
        &self,
        meta: &mut ConstraintSystem<F>,
        key: &'static str,
        pos_vtype_offset_byte: impl FnOnce(
            &mut VirtualCells<'_, F>,
        ) -> (
            Expression<F>,
            Expression<F>,
            Expression<F>,
            Expression<F>,
        ),
        enable: impl FnOnce(&mut VirtualCells<'_, F>) -> Expression<F>,
    ) {
        meta.lookup(key, |meta| {
            let (pos, vtype, offset, byte) = pos_vtype_offset_byte(meta);

            vec![(
                (vtype * constant_from!(1u64 << 32)
                    + offset * constant_from!(1 << 24)
                    + pos * constant_from!(1 << 16)
                    + byte)
                    * enable(meta),
                self.byte_offset_unchanged_validation_col,
            )]
        });
    }

    pub fn configure_in_byte_shift_range(
        &self,
        meta: &mut ConstraintSystem<F>,
        key: &'static str,
        pos_vtype_offset_byte_value: impl Fn(
            &mut VirtualCells<'_, F>,
        ) -> (
            Expression<F>,
            Expression<F>,
            Expression<F>,
            Expression<F>,
            Expression<F>,
        ),
        enable: impl Fn(&mut VirtualCells<'_, F>) -> Expression<F>,
    ) {
        meta.lookup(key, |meta| {
            let (pos, vtype, offset, byte, value) = pos_vtype_offset_byte_value(meta);

            vec![(
                (vtype * constant_from_bn!(&(BigUint::from(1u64) << 88))
                    + offset * constant_from_bn!(&(BigUint::from(1u64) << 80))
                    + pos * constant_from_bn!(&(BigUint::from(1u64) << 72))
                    + byte * constant_from_bn!(&(BigUint::from(1u64) << 64))
                    + value.clone())
                    * enable(meta),
                self.byte_shift_res_col,
            )]
        });

        meta.lookup("bytes shift validation", |meta| {
            let (_, _, _, _, value) = pos_vtype_offset_byte_value(meta);

            vec![(value * enable(meta), self.byte_shift_validation_col)]
        });
    }

    pub fn configure_lt_lookup(
        &self,
        meta: &mut ConstraintSystem<F>,
        key: &'static str,
        left_right_bytes: impl FnOnce(&mut VirtualCells<'_, F>) -> (Expression<F>, Expression<F>),
        enable: impl FnOnce(&mut VirtualCells<'_, F>) -> Expression<F>,
    ) {
        meta.lookup(key, |meta| {
            let (left, right) = left_right_bytes(meta);

            // When prefix is not existing, left = 0 and right = 0 can successfully lookup,
            // since the table contains zero entry.
            let prefix = constant_from!(1 << 16);

            vec![(
                (prefix + left * constant_from!(1 << 8) + right) * enable(meta),
                self.byte_lt_col,
            )]
        });
    }

    pub fn configure_pow_2_lookup(
        &self,
        meta: &mut ConstraintSystem<F>,
        key: &'static str,
        shift: impl FnOnce(&mut VirtualCells<'_, F>) -> (Expression<F>, Expression<F>),
        enable: impl FnOnce(&mut VirtualCells<'_, F>) -> Expression<F>,
    ) {
        meta.lookup(key, |meta| {
            let (result, shift) = shift(meta);

            vec![(
                (result * constant_from!(1 << 16)
                    + shift * constant_from!(1 << 8)
                    + constant_from!(1))
                    * enable(meta),
                self.pow_2_col,
            )]
        });
    }
}

pub struct RangeTableChip<F: FieldExt> {
    config: RangeTableConfig<F>,
    _phantom: PhantomData<F>,
}

pub fn byte_shift(vtype: VarType, offset: usize, pos: usize, byte: u64) -> u64 {
    let size = vtype.byte_size() as usize;
    if pos >= offset && pos < offset + size {
        byte << ((pos - offset) * 8)
    } else {
        0
    }
}

pub fn byte_shift_tbl_encode<F: FieldExt>(
    vtype: VarType,
    offset: usize,
    pos: usize,
    byte: u64,
) -> F {
    let mut bn = BigUint::from(vtype as u64);
    bn = bn << 8usize;
    bn += offset;
    bn = bn << 8usize;
    bn += pos;
    bn = bn << 8usize;
    bn += byte;
    bn = bn << 64usize;
    bn += byte_shift(vtype, offset, pos, byte);
    bn_to_field(&bn)
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
            || "bitop range table",
            |mut table| {
                let mut i = 0;
                for op in BitOp::iter() {
                    for l in 0..(1 << 4) {
                        for r in 0..if op.is_binop() { 1 << 4 } else { 1 } {
                            let res = op.eval(l, r);
                            table.assign_cell(
                                || "range table",
                                self.config.bitop_col,
                                i,
                                || {
                                    Ok(F::from(
                                        ((op.clone() as u64) << 12) | (l << 8) | (r << 4) | res,
                                    ))
                                },
                            )?;
                            i += 1;
                        }
                    }
                }
                Ok(())
            },
        )?;

        layouter.assign_table(
            || "byte shift res table",
            |mut table| {
                let mut index = 0usize;

                for t in VarType::iter() {
                    for offset in 0..8 {
                        for pos in 0..8 {
                            for b in 0..256u64 {
                                table.assign_cell(
                                    || "byte shift res table",
                                    self.config.byte_shift_res_col,
                                    index,
                                    || Ok(byte_shift_tbl_encode::<F>(t, offset, pos, b)),
                                )?;
                                index += 1;
                            }
                        }
                    }
                }

                table.assign_cell(
                    || "byte shift res table",
                    self.config.byte_shift_res_col,
                    index,
                    || Ok(F::zero()),
                )?;

                Ok(())
            },
        )?;

        layouter.assign_table(
            || "byte shift validation table",
            |mut table| {
                let mut index = 0usize;
                for shift in 0..8 {
                    for b in 0..256u64 {
                        table.assign_cell(
                            || "byte shift validation table",
                            self.config.byte_shift_validation_col,
                            index,
                            || Ok(F::from(b << (shift * 8))),
                        )?;
                        index += 1;
                    }
                }

                Ok(())
            },
        )?;

        layouter.assign_table(
            || "byte offset unchangable validation table",
            |mut table| {
                let mut index = 0usize;
                for t in VarType::iter() {
                    for offset in 0..8 {
                        for pos in 0..8 {
                            let (start, end) = if pos >= offset && pos < offset + t.byte_size() {
                                (0, 511u64)
                            } else {
                                (255, 256u64)
                            };
                            for b in start..end {
                                table.assign_cell(
                                    || "byte unchangable table",
                                    self.config.byte_offset_unchanged_validation_col,
                                    index,
                                    || {
                                        Ok(F::from(
                                            ((t as u64) << 32) + (offset << 24) + (pos << 16) + b,
                                        ))
                                    },
                                )?;
                                index += 1;
                            }
                        }
                    }
                }

                table.assign_cell(
                    || "byte shift res table",
                    self.config.byte_offset_unchanged_validation_col,
                    index,
                    || Ok(F::zero()),
                )?;

                Ok(())
            },
        )?;

        layouter.assign_table(
            || "lt table",
            |mut table| {
                let mut offset = 0;

                for i in 1u64..(1 << 8) {
                    for j in 0u64..i {
                        table.assign_cell(
                            || "range table",
                            self.config.byte_lt_col,
                            offset,
                            || Ok(F::from((1 << 16) + (j << 8) + i)),
                        )?;

                        offset += 1;
                    }
                }

                table.assign_cell(
                    || "byte shift res table",
                    self.config.byte_lt_col,
                    offset,
                    || Ok(F::zero()),
                )?;

                Ok(())
            },
        )?;

        layouter.assign_table(
            || "pow 2 table",
            |mut table| {
                let mut offset = 0;

                for i in 0u64..64 {
                    table.assign_cell(
                        || "pow 2 table enable",
                        self.config.pow_2_col,
                        offset,
                        || Ok(F::from(((1 << i) << 16) + (i << 8) + 1)),
                    )?;

                    offset += 1;
                }

                table.assign_cell(
                    || "pow 2 shift disable",
                    self.config.pow_2_col,
                    offset,
                    || Ok(F::zero()),
                )?;

                Ok(())
            },
        )?;

        Ok(())
    }
}
