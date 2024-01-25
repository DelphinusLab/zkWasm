use halo2_proofs::arithmetic::FieldExt;
use halo2_proofs::circuit::Layouter;
use halo2_proofs::plonk::Error;
use specs::etable::EventTable;
use specs::host_function::HostPlugin;
use specs::step::StepInfo;

use crate::foreign::context::Op;

use super::ContextContHelperTableConfig;

pub struct ContextContHelperTableChip<F: FieldExt> {
    pub(crate) config: ContextContHelperTableConfig<F>,
}

impl<F: FieldExt> ContextContHelperTableChip<F> {
    pub fn new(config: ContextContHelperTableConfig<F>) -> Self {
        Self { config }
    }

    pub fn assign(
        &self,
        layouter: &impl Layouter<F>,
        inputs: &Vec<u64>,
        outputs: &Vec<u64>,
    ) -> Result<(), Error> {
        layouter.assign_region(
            || "context cont helper assign",
            |region| {
                for (offset, input) in inputs.iter().enumerate() {
                    region.assign_advice(
                        || "context cont input index",
                        self.config.input,
                        offset + 1, // The first fixed index should be 1.
                        || Ok(F::from(*input)),
                    )?;
                }

                for (offset, output) in outputs.iter().enumerate() {
                    region.assign_advice(
                        || "context cont output index",
                        self.config.output,
                        offset + 1, // The first fixed index should be 1.
                        || Ok(F::from(*output)),
                    )?;
                }

                Ok(())
            },
        )?;

        Ok(())
    }
}

pub trait ExtractContextFromTrace {
    fn get_context_inputs(&self) -> Vec<u64>;
    fn get_context_outputs(&self) -> Vec<u64>;
}

impl ExtractContextFromTrace for EventTable {
    fn get_context_inputs(&self) -> Vec<u64> {
        self.entries()
            .iter()
            .filter_map(|e| match &e.step_info {
                StepInfo::CallHost {
                    plugin: HostPlugin::Context,
                    op_index_in_plugin,
                    ret_val,
                    ..
                } => {
                    if *op_index_in_plugin == Op::ReadContext as usize {
                        *ret_val
                    } else {
                        None
                    }
                }
                _ => None,
            })
            .collect()
    }

    fn get_context_outputs(&self) -> Vec<u64> {
        self.entries()
            .iter()
            .filter_map(|e| match &e.step_info {
                StepInfo::CallHost {
                    plugin: HostPlugin::Context,
                    op_index_in_plugin,
                    args,
                    ..
                } => {
                    if *op_index_in_plugin == Op::WriteContext as usize {
                        Some(args[0])
                    } else {
                        None
                    }
                }
                _ => None,
            })
            .collect()
    }
}
