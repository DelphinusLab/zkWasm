use super::WasmInputHelperTableConfig;
use crate::foreign::wasm_input_helper::circuits::ENABLE_LINES;
use halo2_proofs::arithmetic::FieldExt;
use halo2_proofs::circuit::AssignedCell;
use halo2_proofs::circuit::Layouter;
use halo2_proofs::plonk::Error;

pub struct WasmInputHelperTableChip<F: FieldExt> {
    pub(crate) config: WasmInputHelperTableConfig<F>,
}

impl<F: FieldExt> WasmInputHelperTableChip<F> {
    pub fn new(config: WasmInputHelperTableConfig<F>) -> Self {
        Self { config }
    }

    pub fn assign(
        &self,
        layouter: &mut impl Layouter<F>,
        instances: Vec<AssignedCell<F, F>>,
    ) -> Result<(), Error> {
        for (i, instance) in instances.iter().enumerate() {
            layouter.constrain_instance(instance.cell(), self.config.input, i)?;
        }

        layouter.assign_region(
            || "wasm input helper assign",
            |mut region| {
                let offset = instances.len();
                for i in 0..ENABLE_LINES {
                    region.assign_fixed(
                        || "wasm input helper enable",
                        self.config.enable,
                        i as usize + offset,
                        || Ok(F::one()),
                    )?;

                    region.assign_fixed(
                        || "wasm input index",
                        self.config.index,
                        i + offset,
                        || Ok(F::from(i as u64)),
                    )?;
                }

                Ok(())
            },
        )?;
        Ok(())
    }

    pub fn init(&self, _layouter: &mut impl Layouter<F>) -> Result<(), Error> {
        Ok(())
    }
}
