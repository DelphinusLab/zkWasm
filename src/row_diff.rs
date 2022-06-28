use halo2_proofs::{
    arithmetic::FieldExt,
    plonk::{Advice, Column, ConstraintSystem, Expression, VirtualCells},
    poly::Rotation,
};
use std::marker::PhantomData;

pub struct RowDiffConfig<F: FieldExt> {
    data: Column<Advice>,
    same: Column<Advice>,
    _inv: Column<Advice>,
    _mark: PhantomData<F>,
}

impl<F: FieldExt> RowDiffConfig<F> {
    pub fn configure(
        key: &'static str,
        meta: &mut ConstraintSystem<F>,
        cols: &mut impl Iterator<Item = Column<Advice>>,
    ) -> Self {
        let data = cols.next().unwrap();
        let same = cols.next().unwrap();
        let inv = cols.next().unwrap();
        meta.create_gate(key, |meta| {
            let curr = meta.query_advice(data, Rotation::cur());
            let prev = meta.query_advice(data, Rotation::prev());
            let inv = meta.query_advice(inv, Rotation::cur());
            let same = meta.query_advice(same, Rotation::cur());
            vec![
                (curr.clone() - prev.clone()) * inv.clone()
                    - same.clone()
                    - Expression::Constant(F::one()),
                (curr.clone() - prev.clone()) * same.clone(),
            ]
        });

        RowDiffConfig {
            data,
            same,
            _inv: inv,
            _mark: PhantomData,
        }
    }

    pub fn is_same(&self, meta: &mut VirtualCells<F>) -> Expression<F> {
        meta.query_advice(self.same, Rotation::cur())
    }

    pub fn data(&self, meta: &mut VirtualCells<F>) -> Expression<F> {
        meta.query_advice(self.data, Rotation::cur())
    }

    pub fn diff(&self, meta: &mut VirtualCells<F>) -> Expression<F> {
        let curr = meta.query_advice(self.data, Rotation::cur());
        let prev = meta.query_advice(self.data, Rotation::prev());
        curr - prev
    }
}
