use std::marker::PhantomData;

use halo2_proofs::{arithmetic::Field, circuit::SimpleFloorPlanner, plonk::Circuit};

#[derive(Clone)]
pub struct TestCircuitConfig<F> {
    _data: PhantomData<F>,
}

pub struct TestCircuit<F> {
    _data: PhantomData<F>,
}

impl<F> TestCircuit<F> {
    pub fn new() -> Self {
        todo!()
    }
}

impl<F: Field> Circuit<F> for TestCircuit<F> {
    type Config = TestCircuitConfig<F>;

    type FloorPlanner = SimpleFloorPlanner;

    fn without_witnesses(&self) -> Self {
        todo!()
    }

    fn configure(meta: &mut halo2_proofs::plonk::ConstraintSystem<F>) -> Self::Config {
        todo!()
    }

    fn synthesize(
        &self,
        config: Self::Config,
        layouter: impl halo2_proofs::circuit::Layouter<F>,
    ) -> Result<(), halo2_proofs::plonk::Error> {
        todo!()
    }
}
