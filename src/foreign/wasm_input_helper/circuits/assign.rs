use crate::foreign::wasm_input_helper::RESERVED_INSTANCES_NUMBER;

use super::WasmInputHelperTableConfig;
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
        assert_eq!(instances.len(), RESERVED_INSTANCES_NUMBER);

        for (i, instance) in instances.iter().enumerate() {
            layouter.constrain_instance(instance.cell(), self.config.input, i)?;
        }

        Ok(())
    }
}
