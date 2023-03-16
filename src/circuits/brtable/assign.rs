use halo2_proofs::arithmetic::FieldExt;
use halo2_proofs::circuit::Layouter;
use halo2_proofs::plonk::Error;
use specs::brtable::BrTable;
use specs::brtable::ElemTable;

use super::BrTableChip;
use crate::circuits::utils::bn_to_field;

impl<F: FieldExt> BrTableChip<F> {
    pub(in crate::circuits) fn assign(
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

                for e in br_table_init.entries() {
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
