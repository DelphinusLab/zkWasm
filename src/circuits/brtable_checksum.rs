use crate::circuits::traits::ConfigureLookupTable;
use crate::circuits::utils::bn_to_field;
use crate::curr;
use halo2_proofs::arithmetic::FieldExt;
use halo2_proofs::circuit::AssignedCell;
use halo2_proofs::circuit::Layouter;
use halo2_proofs::plonk::Advice;
use halo2_proofs::plonk::Column;
use halo2_proofs::plonk::ConstraintSystem;
use halo2_proofs::plonk::Error;
use halo2_proofs::plonk::Expression;
use halo2_proofs::plonk::VirtualCells;
use specs::brtable::BrTable;
use specs::brtable::ElemTable;
use std::marker::PhantomData;

use super::config::max_brtable_rows;

#[derive(Clone)]
pub struct BrTableConfig<F: FieldExt> {
    col: Column<Advice>,
    _mark: PhantomData<F>,
}

pub struct BrTableChip<F: FieldExt> {
    config: BrTableConfig<F>,
}

impl<F: FieldExt> BrTableChip<F> {
    pub fn new(config: BrTableConfig<F>) -> Self {
        BrTableChip { config }
    }
}

impl<F: FieldExt> BrTableConfig<F> {
    pub(in crate::circuits) fn configure(meta: &mut ConstraintSystem<F>) -> Self {
        let col = meta.advice_column();
        meta.enable_equality(col);
        Self {
            col,
            _mark: PhantomData,
        }
    }
}

impl<F: FieldExt> ConfigureLookupTable<F> for BrTableConfig<F> {
    fn configure_in_table(
        &self,
        meta: &mut ConstraintSystem<F>,
        key: &'static str,
        expr: impl FnOnce(&mut VirtualCells<'_, F>) -> Expression<F>,
    ) {
        meta.lookup_any(key, |meta| vec![(expr(meta), curr!(meta, self.col))]);
    }
}

impl<F: FieldExt> BrTableChip<F> {
    pub(in crate::circuits) fn assign(
        self,
        layouter: &mut impl Layouter<F>,
        br_table_init: &BrTable,
        elem_table: &ElemTable,
    ) -> Result<Vec<AssignedCell<F, F>>, Error> {
        layouter.assign_region(
            || "br table",
            |mut table| {
                let mut ret = vec![];
                let mut offset = 0;

                for e in br_table_init.entries() {
                    let cell = table.assign_advice(
                        || "br table init cell",
                        self.config.col,
                        offset,
                        || Ok(bn_to_field::<F>(&e.encode())),
                    )?;

                    ret.push(cell);
                    offset += 1;
                }

                for e in elem_table.entries() {
                    let cell = table.assign_advice(
                        || "br table call indirect cell",
                        self.config.col,
                        offset,
                        || Ok(bn_to_field::<F>(&e.encode())),
                    )?;

                    ret.push(cell);
                    offset += 1;
                }

                let max_rows = max_brtable_rows() as usize;
                assert!(offset < max_rows);

                while offset < max_rows {
                    let cell = table.assign_advice(
                        || "br table padding",
                        self.config.col,
                        offset,
                        || Ok(F::zero()),
                    )?;

                    ret.push(cell);
                    offset += 1;
                }

                Ok(ret)
            },
        )
    }
}
