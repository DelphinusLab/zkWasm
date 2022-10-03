use super::{Sha256HelperTableConfig, Sha2HelperEncode, BLOCK_LINES, ENABLE_LINES};
use crate::foreign::sha256_helper::Sha256HelperOp;
use halo2_proofs::{arithmetic::FieldExt, circuit::Layouter, plonk::Error};

pub struct Sha256HelperTableChip<F: FieldExt> {
    config: Sha256HelperTableConfig<F>,
}

//TODO
impl<F: FieldExt> Sha256HelperTableChip<F> {
    pub fn new(config: Sha256HelperTableConfig<F>) -> Self {
        Self { config }
    }
    pub fn assign(
        &self,
        layouter: &mut impl Layouter<F>,
        _entry: Vec<(Sha256HelperOp, Vec<u32>)>,
    ) -> Result<(), Error> {
        layouter.assign_region(
            || "sha256 helper assign",
            |mut region| {
                for i in 0..ENABLE_LINES {
                    region.assign_fixed(
                        || "sha256 helper sel",
                        self.config.sel,
                        i as usize,
                        || Ok(F::one()),
                    )?;

                    region.assign_fixed(
                        || "sha256 helper first block line sel",
                        self.config.sel,
                        i as usize,
                        || {
                            Ok(if i % BLOCK_LINES == 0 {
                                F::one()
                            } else {
                                F::zero()
                            })
                        },
                    )?;
                }

                Ok(())
            },
        )?;
        Ok(())
    }

    pub fn init(&self, layouter: &mut impl Layouter<F>) -> Result<(), Error> {
        layouter.assign_table(
            || "sha256 helper table",
            |mut table| {
                table.assign_cell(
                    || "sha256 helper table",
                    self.config.op_valid_set,
                    0,
                    || Ok(F::zero()),
                )?;
                let mut index = 1;

                for a in 0..1 << 4 {
                    for b in 0..1 << 4 {
                        for c in 0..1 << 4 {
                            for op in vec![
                                Sha256HelperOp::SSigma0,
                                Sha256HelperOp::SSigma1,
                                Sha256HelperOp::LSigma0,
                                Sha256HelperOp::LSigma1,
                            ] {
                                table.assign_cell(
                                    || "sha256 helper table",
                                    self.config.op_valid_set,
                                    index,
                                    || {
                                        Ok(Sha2HelperEncode::encode_opcocde_f::<F>(
                                            op,
                                            vec![a, b, c, a ^ b ^ c],
                                        ))
                                    },
                                )?;
                                index += 1;
                            }

                            table.assign_cell(
                                || "sha256 helper table",
                                self.config.op_valid_set,
                                index,
                                || {
                                    Ok(Sha2HelperEncode::encode_opcocde_f::<F>(
                                        Sha256HelperOp::Ch,
                                        vec![a, b, c, (a & b) ^ (!a & c)],
                                    ))
                                },
                            )?;
                            index += 1;

                            table.assign_cell(
                                || "sha256 helper table",
                                self.config.op_valid_set,
                                index,
                                || {
                                    Ok(Sha2HelperEncode::encode_opcocde_f::<F>(
                                        Sha256HelperOp::Maj,
                                        vec![a, b, c, (a & b) ^ (a & c) ^ (b & c)],
                                    ))
                                },
                            )?;
                            index += 1;
                        }
                    }
                }

                Ok(())
            },
        )?;

        Ok(())
    }
}
