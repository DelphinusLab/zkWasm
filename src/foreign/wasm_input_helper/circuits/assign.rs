use super::WasmInputHelperTableConfig;
use crate::foreign::wasm_input_helper::circuits::ENABLE_LINES;
use halo2_proofs::{arithmetic::FieldExt, circuit::Layouter, plonk::Error};
use specs::etable::EventTableEntry;

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
        _entries: &Vec<EventTableEntry>,
    ) -> Result<(), Error> {
        layouter.assign_region(
            || "wasm input helper assign",
            |mut region| {
                for i in 0..ENABLE_LINES {
                    region.assign_fixed(
                        || "wasm input helper enable",
                        self.config.enable,
                        i as usize,
                        || Ok(F::one()),
                    )?;

                    region.assign_fixed(
                        || "wasm input index",
                        self.config.index,
                        i,
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
