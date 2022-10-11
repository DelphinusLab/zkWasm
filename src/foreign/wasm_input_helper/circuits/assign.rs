use super::WasmInputHelperTableConfig;
use crate::foreign::wasm_input_helper::circuits::ENABLE_LINES;
use halo2_proofs::{arithmetic::FieldExt, circuit::Layouter, plonk::Error};
use specs::{etable::EventTableEntry, host_function::HostPlugin, step::StepInfo};

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
        entries: &Vec<EventTableEntry>,
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

                let mut offset = 0;

                for entry in entries.iter() {
                    if let StepInfo::CallHost {
                        plugin,
                        args,
                        ret_val,
                        ..
                    } = &entry.step_info
                    {
                        assert_eq!(*plugin, HostPlugin::HostInput);

                        // is public
                        if args[0] == 1 {
                            let mut input = ret_val.unwrap();

                            for i in 0..8 {
                                region.assign_advice(
                                    || "wasm input u8 cells",
                                    self.config.input_u8[i],
                                    offset,
                                    || Ok(F::from(input & 0xff)),
                                )?;

                                input >>= 8;
                            }

                            offset += 1;
                        }
                    } else {
                        unreachable!()
                    }
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
