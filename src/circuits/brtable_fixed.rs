use crate::circuits::traits::ConfigureLookupTable;
use crate::circuits::utils::bn_to_field;
use halo2_proofs::arithmetic::FieldExt;
use halo2_proofs::circuit::Layouter;
use halo2_proofs::plonk::ConstraintSystem;
use halo2_proofs::plonk::Error;
use halo2_proofs::plonk::Expression;
use halo2_proofs::plonk::TableColumn;
use halo2_proofs::plonk::VirtualCells;
use specs::brtable::BrTable;
use specs::brtable::ElemTable;
use std::marker::PhantomData;

#[derive(Clone)]
pub struct BrTableConfig<F: FieldExt> {
    pub(self) col: TableColumn,
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
        Self {
            col: meta.lookup_table_column(),
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
        meta.lookup(key, |meta| vec![(expr(meta), self.col)]);
    }
}

impl<F: FieldExt> BrTableChip<F> {
    pub(in crate::circuits) fn assign(
        self,
        layouter: &mut impl Layouter<F>,
        br_table_init: &BrTable,
        elem_table: &ElemTable,
    ) -> Result<(), Error> {
        layouter.assign_table(
            || "br table",
            |mut table| {
                table.assign_cell(
                    || "br table empty cell",
                    self.config.col,
                    0,
                    || Ok(F::zero()),
                )?;

                let mut offset = 1;

                for e in br_table_init.entries() {
                    table.assign_cell(
                        || "br table init cell",
                        self.config.col,
                        offset,
                        || Ok(bn_to_field::<F>(&e.encode())),
                    )?;

                    offset += 1;
                }

                for e in elem_table.entries() {
                    table.assign_cell(
                        || "call indirect init cell",
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
