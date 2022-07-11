use std::marker::PhantomData;

use halo2_proofs::{
    arithmetic::FieldExt,
    circuit::{Layouter, SimpleFloorPlanner},
    plonk::{Circuit, ConstraintSystem, Error},
};

use crate::{
    builder::{CircuitBuilder, VAR_COLUMNS},
    etable::{EventTableChip, EventTableConfig},
    itable::{InstTableChip, InstTableConfig},
    jtable::JumpTableConfig,
    mtable::MemoryTableConfig,
};

#[derive(Clone)]
pub struct TestCircuitConfig<F: FieldExt> {
    etable: EventTableConfig<F>,
    itable: InstTableConfig<F>,
    jtable: JumpTableConfig<F>,
    mtable: MemoryTableConfig<F>,
}

#[derive(Default)]
pub struct TestCircuit<F: FieldExt> {
    circuit_builder: CircuitBuilder,
    _data: PhantomData<F>,
}

impl<F: FieldExt> TestCircuit<F> {
    pub fn new(builder: &CircuitBuilder) -> Self {
        TestCircuit {
            circuit_builder: builder.clone(),
            _data: PhantomData,
        }
    }
}

impl<F: FieldExt> Circuit<F> for TestCircuit<F> {
    type Config = TestCircuitConfig<F>;

    type FloorPlanner = SimpleFloorPlanner;

    fn without_witnesses(&self) -> Self {
        Self::default()
    }

    fn configure(meta: &mut ConstraintSystem<F>) -> Self::Config {
        let mut cols = [(); VAR_COLUMNS].map(|_| meta.advice_column()).into_iter();
        let itable = InstTableConfig::new(meta);
        let jtable = JumpTableConfig::new(&mut cols);
        let mtable = MemoryTableConfig::new(meta, &mut cols);
        let etable = EventTableConfig::new(meta, &mut cols, &itable, &mtable, &jtable);

        Self::Config {
            etable,
            itable,
            jtable,
            mtable,
        }
    }

    fn synthesize(
        &self,
        config: Self::Config,
        mut layouter: impl Layouter<F>,
    ) -> Result<(), Error> {
        let _echip = EventTableChip::new(config.etable);
        let ichip = InstTableChip::new(config.itable);

        ichip.add_inst_init(&mut layouter, &self.circuit_builder.itable)?;

        Ok(())
    }
}
