use std::marker::PhantomData;

use halo2_proofs::{
    arithmetic::FieldExt,
    plonk::{Advice, Column, ConstraintSystem, Fixed},
};

use crate::{constant_from, curr, fixed_curr};

use super::rtable::RangeTableConfig;

pub struct BitTableConfig<F: FieldExt> {
    step_sel: Column<Fixed>,
    sel: Column<Fixed>,
    value: Column<Advice>,
    _mark: PhantomData<F>,
}

impl<F: FieldExt> BitTableConfig<F> {
    pub(crate) fn configure(meta: &mut ConstraintSystem<F>, rtable: RangeTableConfig<F>) -> Self {
        let step_sel = meta.fixed_column();
        let sel = meta.fixed_column();
        let value = meta.advice_column();

        rtable.configure_in_u8_range(meta, "bit table u8", |meta| {
            (constant_from!(1) - fixed_curr!(meta, step_sel)) * curr!(meta, value)
        });

        // TODO

        Self {
            step_sel,
            sel,
            value,
            _mark: PhantomData,
        }
    }
}
