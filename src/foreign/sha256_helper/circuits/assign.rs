use super::{Sha256HelperTableConfig, Sha2HelperEncode, BLOCK_LINES, OP_ARGS_NUM};
use crate::{
    circuits::{rtable::RangeTableMixColumn, utils::Context},
    foreign::sha256_helper::Sha256HelperOp,
};
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
        ctx: &mut Context<'_, F>,
        entry: &Vec<EventTableEntry>,
    ) -> Result<(), Error> {
        assert!(ctx.start_offset % BLOCK_LINES == 0);

        for i in ctx.start_offset..ctx.end_offset {
            ctx.region.as_ref().borrow_mut().assign_fixed(
                || "sha256 helper sel",
                self.config.sel,
                i as usize,
                || Ok(F::one()),
            )?;

            if i % BLOCK_LINES == 0 {
                ctx.region.as_ref().borrow_mut().assign_fixed(
                    || "sha256 helper first block line sel",
                    self.config.block_first_line_sel,
                    i as usize,
                    || Ok(F::one()),
                )?;
            }

            // limited to u8 except for block first line
            if i % BLOCK_LINES != 0 {
                self.config.aux.assign_lookup(
                    &mut ctx.region.as_ref().borrow_mut(),
                    i,
                    RangeTableMixColumn::U8,
                )?;
            }
        }

        // op args ret
        for (block_i, step) in entry.iter().enumerate() {
            if let StepInfo::CallHost {
                plugin,
                function_name,
                args,
                ret_val,
                ..
            } = &step.step_info
            {
                assert_eq!(*plugin, HostPlugin::Sha256);

                let offset = ctx.start_offset + block_i * BLOCK_LINES;
                let op = Sha256HelperOp::from(function_name);
                let args: Vec<u32> = args.iter().map(|arg| *arg as u32).collect();
                let ret = ret_val.unwrap() as u32;

                for i in 0..BLOCK_LINES {
                    ctx.region.as_ref().borrow_mut().assign_advice(
                        || "sha256 helper table",
                        self.config.op,
                        offset + i,
                        || Ok(F::from(op as u64)),
                    )?;
                }

                ctx.region.as_ref().borrow_mut().assign_advice(
                    || "sha256 helper opcode",
                    self.config.aux.internal,
                    offset,
                    || Ok(Sha2HelperEncode::encode_opcode_f(op, &args, ret)),
                )?;

                ctx.region.as_ref().borrow_mut().assign_advice(
                    || "sha256 helper enable",
                    self.config.op_bit.0,
                    offset,
                    || Ok(F::from(1u64)),
                )?;

                ctx.region.as_ref().borrow_mut().assign_advice(
                    || "sha256 helper op bit",
                    self.config.op_bit.0,
                    offset + (op as usize),
                    || Ok(F::from(1u64)),
                )?;

                let start = match op {
                    Sha256HelperOp::SSigma0
                    | Sha256HelperOp::SSigma1
                    | Sha256HelperOp::LSigma0
                    | Sha256HelperOp::LSigma1 => 0,
                    Sha256HelperOp::Ch | Sha256HelperOp::Maj => 1,
                };

                for (arg_i, arg) in args.iter().enumerate() {
                    for i in 0..8 {
                        ctx.region.as_ref().borrow_mut().assign_advice(
                            || "sha256 helper args",
                            self.config.args[arg_i + start],
                            offset + i,
                            || Ok(F::from((arg >> (i * 4)) as u64 & 0xfu64)),
                        )?;
                    }
                }

                for i in 0..8 {
                    ctx.region.as_ref().borrow_mut().assign_advice(
                        || "sha256 helper ret",
                        self.config.args[OP_ARGS_NUM - 1],
                        offset + i,
                        || Ok(F::from((ret >> (i * 4)) as u64 & 0xfu64)),
                    )?;
                }

                match op {
                    Sha256HelperOp::Ch => {
                        self.assign_ch(&mut ctx.region.as_ref().borrow_mut(), offset, &args)?
                    }
                    Sha256HelperOp::Maj => {
                        self.assign_maj(&mut ctx.region.as_ref().borrow_mut(), offset, &args)?
                    }
                    Sha256HelperOp::LSigma0 => {
                        self.assign_lsigma0(&mut ctx.region.as_ref().borrow_mut(), offset, &args)?
                    }
                    Sha256HelperOp::LSigma1 => {
                        self.assign_lsigma1(&mut ctx.region.as_ref().borrow_mut(), offset, &args)?
                    }
                    Sha256HelperOp::SSigma0 => {
                        self.assign_ssigma0(&mut ctx.region.as_ref().borrow_mut(), offset, &args)?
                    }
                    Sha256HelperOp::SSigma1 => {
                        self.assign_ssigma1(&mut ctx.region.as_ref().borrow_mut(), offset, &args)?
                    }
                }
            } else {
                unreachable!()
            }
        }

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
                                        Ok(Sha2HelperEncode::encode_table_f::<F>(
                                            op,
                                            [a, b, c],
                                            (a ^ b ^ c) & 0xf,
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
                                    Ok(Sha2HelperEncode::encode_table_f::<F>(
                                        Sha256HelperOp::Ch,
                                        [a, b, c],
                                        ((a & b) ^ (!a & c)) & 0xf,
                                    ))
                                },
                            )?;
                            index += 1;

                            table.assign_cell(
                                || "sha256 helper table",
                                self.config.op_valid_set,
                                index,
                                || {
                                    Ok(Sha2HelperEncode::encode_table_f::<F>(
                                        Sha256HelperOp::Maj,
                                        [a, b, c],
                                        ((a & b) ^ (a & c) ^ (b & c)) & 0xf,
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
