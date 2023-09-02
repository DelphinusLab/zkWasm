use halo2_proofs::arithmetic::FieldExt;
use halo2_proofs::circuit::Cell;
use halo2_proofs::circuit::Layouter;
use halo2_proofs::plonk::Error;

use super::ImageTableChip;
use super::ImageTableLayouter;
use crate::circuits::utils::Context;

impl<F: FieldExt> ImageTableChip<F> {
    pub fn assign(
        self,
        layouter: &mut impl Layouter<F>,
        image_table: ImageTableLayouter<F>,
        permutation_cells: ImageTableLayouter<Cell>,
    ) -> Result<(), Error> {
        layouter.assign_region(
            || "image table",
            |region| {
                let mut ctx = Context::new(region);

                let entry_fid_cell = ctx.region.assign_advice(
                    || "image table: entry fid",
                    self.config.col,
                    ctx.offset,
                    || Ok(image_table.entry_fid),
                )?;
                ctx.region
                    .constrain_equal(permutation_cells.entry_fid, entry_fid_cell.cell())?;

                ctx.next();

                for (static_frame_entry, cell_in_frame_table) in image_table
                    .static_frame_entries
                    .iter()
                    .zip(permutation_cells.static_frame_entries.iter())
                {
                    let cell = ctx
                        .region
                        .assign_advice(
                            || "image table: entry fid enable",
                            self.config.col,
                            ctx.offset,
                            || Ok(static_frame_entry.0),
                        )?
                        .cell();
                    ctx.region.constrain_equal(cell, cell_in_frame_table.0)?;

                    ctx.next();

                    let cell = ctx
                        .region
                        .assign_advice(
                            || "image table: entry fid entry",
                            self.config.col,
                            ctx.offset,
                            || Ok(static_frame_entry.1),
                        )?
                        .cell();
                    ctx.region.constrain_equal(cell, cell_in_frame_table.1)?;

                    ctx.next();
                }

                for value in image_table.lookup_entries.as_ref().unwrap() {
                    ctx.region
                        .assign_advice(
                            || "image table",
                            self.config.col,
                            ctx.offset,
                            || Ok(*value),
                        )?
                        .cell();

                    ctx.next();
                }

                Ok(())
            },
        )
    }
}
