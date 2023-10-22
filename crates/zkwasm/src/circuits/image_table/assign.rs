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

                macro_rules! assign_one_line {
                    ($v: expr) => {{
                        let cell = ctx
                            .region
                            .assign_advice(
                                || "image table",
                                self.config.col,
                                ctx.offset,
                                || Ok($v),
                            )?
                            .cell();

                        ctx.next();

                        cell
                    }};
                }

                // FIXME: should not constrain equal if continuation is enabled.
                let entry_fid_cell = assign_one_line!(image_table.entry_fid);
                ctx.region
                    .constrain_equal(permutation_cells.entry_fid, entry_fid_cell)?;

                let initial_memory_pages_cell = assign_one_line!(image_table.initial_memory_pages);
                // FIXME: should not constrain equal if continuation is enabled.
                ctx.region.constrain_equal(
                    permutation_cells.initial_memory_pages,
                    initial_memory_pages_cell,
                )?;

                let maximal_memory_pages_cell = assign_one_line!(image_table.maximal_memory_pages);
                ctx.region.constrain_equal(
                    permutation_cells.maximal_memory_pages,
                    maximal_memory_pages_cell,
                )?;

                for (static_frame_entry, cell_in_frame_table) in image_table
                    .static_frame_entries
                    .iter()
                    .zip(permutation_cells.static_frame_entries.iter())
                {
                    // Enable cell
                    let cell = assign_one_line!(static_frame_entry.0);
                    ctx.region.constrain_equal(cell, cell_in_frame_table.0)?;

                    let cell = assign_one_line!(static_frame_entry.1);
                    ctx.region.constrain_equal(cell, cell_in_frame_table.1)?;
                }

                for value in image_table.lookup_entries.as_ref().unwrap() {
                    assign_one_line!(*value);
                }

                Ok(())
            },
        )
    }
}
