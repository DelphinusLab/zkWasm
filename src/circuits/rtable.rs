use crate::constant_from;
use halo2_proofs::arithmetic::FieldExt;
use halo2_proofs::circuit::Layouter;
use halo2_proofs::plonk::ConstraintSystem;
use halo2_proofs::plonk::Error;
use halo2_proofs::plonk::Expression;
use halo2_proofs::plonk::TableColumn;
use halo2_proofs::plonk::VirtualCells;
use specs::itable::BitOp;
use specs::mtable::VarType;
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
    // compose_of(byte_pos_of_8byte, var_type, byte) to avoid overflow, 3 + 3 + 8 = 14 bits in total
    vtype_byte_col: TableColumn,
    // op | left | right | res
    bitop_col: TableColumn,
    _mark: PhantomData<F>,
}

impl<F: FieldExt> RangeTableConfig<F> {
    pub fn configure(cols: [TableColumn; 5]) -> Self {
        RangeTableConfig {
            u16_col: cols[0],
            u8_col: cols[1],
            u4_col: cols[2],
            vtype_byte_col: cols[3],
            bitop_col: cols[4],
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
}

pub struct RangeTableChip<F: FieldExt> {
    config: RangeTableConfig<F>,
    _phantom: PhantomData<F>,
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
                                || Ok(F::from(((op.clone() as u64) << 12) | (l << 8) | (r << 4) | res)),
                            )?;
                            i += 1;
                        }
                    }
                }
                Ok(())
            },
        )?;

        layouter.assign_table(
            || "vtype byte validation table",
            |mut table| {
                let mut index = 0usize;

                for pos in 0..8u64 {
                    for t in VarType::iter() {
                        let range = if pos < t.byte_size() { 256u64 } else { 1u64 };
                        //let range = 256;
                        for v in 0..range {
                            table.assign_cell(
                                || "vtype byte validation table",
                                self.config.vtype_byte_col,
                                index,
                                || Ok(F::from((pos << 12) + ((t as u64) << 8) + v)),
                            )?;
                            index += 1;
                        }
                    }
                }

                table.assign_cell(
                    || "vtype byte validation table",
                    self.config.vtype_byte_col,
                    index,
                    || Ok(F::zero()),
                )?;

                Ok(())
            },
        )?;

        Ok(())
    }
}
