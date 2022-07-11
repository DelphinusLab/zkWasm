use std::marker::PhantomData;

use halo2_proofs::{
    arithmetic::FieldExt,
    circuit::{Layouter, SimpleFloorPlanner},
    plonk::{Circuit, ConstraintSystem, Error},
};

use crate::{
    builder::VAR_COLUMNS, etable::EventTableConfig, itable::InstTableConfig,
    jtable::JumpTableConfig, mtable::MemoryTableConfig,
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
    _data: PhantomData<F>,
}

impl<F: FieldExt> TestCircuit<F> {
    pub fn new() -> Self {
        TestCircuit { _data: PhantomData }
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

    fn synthesize(&self, config: Self::Config, layouter: impl Layouter<F>) -> Result<(), Error> {
        todo!()
    }
}
