use halo2_proofs::{arithmetic::FieldExt, circuit::Layouter, plonk::Error};
use specs::external_host_call_table::ExternalHostCallTable;

use crate::circuits::traits::TableSize;

use super::ExternalHostCallChip;

impl<F: FieldExt> ExternalHostCallChip<F> {
    pub(in crate::circuits) fn assign(
        self,
        layouter: &mut impl Layouter<F>,
        table: &ExternalHostCallTable,
    ) -> Result<(), Error> {
        layouter.assign_region(
            || "foreign table",
            |mut region| {
                // Assign Fixed Column
                {
                    for offset in 0..self.config.max_available_size() {
                        region.assign_fixed(
                            || "external host call idx",
                            self.config.idx,
                            offset,
                            || Ok(F::from(offset as u64)),
                        )?;
                    }
                }

                // Assign Advice Columns
                {
                    let mut offset = 0;

                    {
                        region.assign_advice(
                            || "external host call op",
                            self.config.op,
                            offset,
                            || Ok(F::zero()),
                        )?;

                        region.assign_advice(
                            || "external host call value",
                            self.config.arg,
                            offset,
                            || Ok(F::zero()),
                        )?;

                        region.assign_advice(
                            || "external host call is ret",
                            self.config.is_ret,
                            offset,
                            || Ok(F::zero()),
                        )?;
                    }

                    offset += 1;

                    for entry in table.entries() {
                        region.assign_advice(
                            || "external host call op",
                            self.config.op,
                            offset,
                            || Ok(F::from(*entry.op as u64)),
                        )?;

                        region.assign_advice(
                            || "external host call is ret",
                            self.config.is_ret,
                            offset,
                            || Ok(F::from(entry.sig.is_ret() as u64)),
                        )?;

                        region.assign_advice(
                            || "external host call value",
                            self.config.arg,
                            offset,
                            || Ok(F::from(entry.value)),
                        )?;

                        offset += 1;
                    }
                }

                Ok(())
            },
        )?;

        Ok(())
    }
}
