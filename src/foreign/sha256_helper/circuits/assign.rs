use super::{Sha256HelperTableConfig, Sha2HelperEncode, BLOCK_LINES, ENABLE_LINES, OP_ARGS_NUM};
use crate::foreign::sha256_helper::Sha256HelperOp;
use halo2_proofs::{arithmetic::FieldExt, circuit::Layouter, plonk::Error};
use specs::{etable::EventTableEntry, host_function::HostPlugin, step::StepInfo};

pub struct Sha256HelperTableChip<F: FieldExt> {
    pub(crate) config: Sha256HelperTableConfig<F>,
}

impl<F: FieldExt> Sha256HelperTableChip<F> {
    pub fn new(config: Sha256HelperTableConfig<F>) -> Self {
        Self { config }
    }
    pub fn assign(
        &self,
        layouter: &mut impl Layouter<F>,
        entry: &Vec<EventTableEntry>,
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

                // op args ret
                for (block_i, step) in entry.iter().enumerate() {
                    if let StepInfo::CallHost {
                        plugin,
                        host_function_idx,
                        function_name,
                        signature,
                        args,
                        ret_val,
                    } = &step.step_info
                    {
                        assert_eq!(*plugin, HostPlugin::Sha256);

                        // WARNING! host_function_idx may not start from 0.
                        todo!();

                        /*
                        let offset = block_i * BLOCK_LINES;
                        for i in 0..BLOCK_LINES {
                            region.assign_advice(
                                || "sha256 helper table",
                                self.config.op.0,
                                offset + i,
                                || Ok(F::from(*op as u64)),
                            )?;
                        }

                        region.assign_advice(
                            || "sha256 helper enable",
                            self.config.op_bit.0,
                            offset,
                            || Ok(F::from(1u64)),
                        )?;

                        region.assign_advice(
                            || "sha256 helper op bit",
                            self.config.op_bit.0,
                            offset + *op as usize,
                            || Ok(F::from(1u64)),
                        )?;

                        for (arg_i, arg) in args.iter().enumerate() {
                            for i in 0..BLOCK_LINES {
                                region.assign_advice(
                                    || "sha256 helper args",
                                    self.config.args[arg_i].0,
                                    offset + i,
                                    || Ok(F::from((arg >> (i * 4)) as u64 & 0xfu64)),
                                )?;
                            }
                        }

                        for i in 0..BLOCK_LINES {
                            region.assign_advice(
                                || "sha256 helper ret",
                                self.config.args[OP_ARGS_NUM - 1].0,
                                offset + i,
                                || Ok(F::from((ret >> (i * 4)) as u64 & 0xfu64)),
                            )?;
                        }

                        if *op == Sha256HelperOp::LSigma0 as u32 {
                            self.assign_lsigma0(&mut region, offset, args)?;
                        }

                        if *op == Sha256HelperOp::LSigma1 as u32 {
                            self.assign_lsigma1(&mut region, offset, args)?;
                        }

                        if *op == Sha256HelperOp::SSigma0 as u32 {
                            self.assign_ssigma0(&mut region, offset, args)?;
                        }

                        if *op == Sha256HelperOp::SSigma1 as u32 {
                            self.assign_ssigma1(&mut region, offset, args)?;
                        }
                        */
                    } else {
                        unreachable!()
                    }
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
