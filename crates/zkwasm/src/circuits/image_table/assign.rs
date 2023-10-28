use halo2_proofs::arithmetic::FieldExt;
use halo2_proofs::circuit::Layouter;
use halo2_proofs::plonk::Error;

use super::ImageTableChip;
use super::ImageTableLayouter;

impl<F: FieldExt> ImageTableChip<F> {
    pub fn assign(
        self,
        layouter: &mut impl Layouter<F>,
        image_table: ImageTableLayouter<F>,
    ) -> Result<(), Error> {
        layouter.assign_table(
            || "image table",
            |mut table| {
                let mut offset = 0;

                macro_rules! assign_one_line {
                    ($v: expr) => {{
                        table.assign_cell(|| "image table", self.config.col, offset, || Ok($v))?;

                        offset += 1;
                    }};
                }

                for value in image_table.lookup_entries.as_ref().unwrap() {
                    assign_one_line!(*value);
                }

                Ok(())
            },
        )
    }
}
