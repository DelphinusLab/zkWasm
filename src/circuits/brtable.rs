use super::{utils::bn_to_field, Encode};
use halo2_proofs::{
    arithmetic::FieldExt,
    circuit::Layouter,
    plonk::{ConstraintSystem, Error, Expression, TableColumn, VirtualCells},
};
use num_bigint::BigUint;
use specs::{
    brtable::{BrTable, BrTableEntry, ElemTable},
    encode::table::encode_br_table_entry,
};
use std::marker::PhantomData;

impl Encode for BrTableEntry {
    fn encode(&self) -> BigUint {
        encode_br_table_entry(
            BigUint::from(self.moid),
            BigUint::from(self.fid),
            BigUint::from(self.iid),
            BigUint::from(self.index),
            BigUint::from(self.drop),
            BigUint::from(self.keep),
            BigUint::from(self.dst_pc),
        )
    }
}

#[derive(Clone)]
pub struct BrTableConfig<F: FieldExt> {
    col: TableColumn,
    _mark: PhantomData<F>,
}

impl<F: FieldExt> BrTableConfig<F> {
    pub fn configure(col: TableColumn) -> Self {
        Self {
            col,
            _mark: PhantomData,
        }
    }

    pub fn encode(
        &self,
        moid: Expression<F>,
        fid: Expression<F>,
        iid: Expression<F>,
        index: Expression<F>,
        drop: Expression<F>,
        keep: Expression<F>,
        dst_pc: Expression<F>,
    ) -> Expression<F> {
        encode_br_table_entry(moid, fid, iid, index, drop, keep, dst_pc)
    }

    pub fn configure_in_table(
        &self,
        meta: &mut ConstraintSystem<F>,
        key: &'static str,
        expr: impl FnOnce(&mut VirtualCells<'_, F>) -> Expression<F>,
    ) {
        meta.lookup(key, |meta| vec![(expr(meta), self.col)]);
    }
}

pub struct BrTableChip<F: FieldExt> {
    config: BrTableConfig<F>,
}

impl<F: FieldExt> BrTableChip<F> {
    pub fn new(config: BrTableConfig<F>) -> Self {
        BrTableChip { config }
    }

    pub fn assign(
        self,
        layouter: &mut impl Layouter<F>,
        br_table_init: &BrTable,
        elem_table: &ElemTable,
    ) -> Result<(), Error> {
        layouter.assign_table(
            || "minit",
            |mut table| {
                table.assign_cell(|| "brtable init", self.config.col, 0, || Ok(F::zero()))?;

                let mut offset = 1;

                for e in br_table_init.entries().iter() {
                    table.assign_cell(
                        || "brtable init",
                        self.config.col,
                        offset,
                        || Ok(bn_to_field::<F>(&e.encode())),
                    )?;

                    offset += 1;
                }

                for e in elem_table.entries() {
                    table.assign_cell(
                        || "call indirect init",
                        self.config.col,
                        offset,
                        || Ok(bn_to_field::<F>(&e.encode())),
                    )?;

                    offset += 1;
                }

                Ok(())
            },
        )?;

        Ok(())
    }
}
