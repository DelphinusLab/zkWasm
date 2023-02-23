use std::collections::BTreeMap;

use halo2_proofs::{
    arithmetic::FieldExt,
    plonk::{ConstraintSystem, Expression, VirtualCells},
};

pub(super) struct ConstraintBuilder<'a, F: FieldExt> {
    meta: &'a mut ConstraintSystem<F>,
    pub(in crate::circuits::etable_v2) constraints: Vec<(
        &'static str,
        Box<dyn FnOnce(&mut VirtualCells<F>) -> Vec<Expression<F>>>,
    )>,
    pub(in crate::circuits::etable_v2) lookups: BTreeMap<
        &'static str,
        Vec<(
            &'static str,
            Box<dyn Fn(&mut VirtualCells<F>) -> Expression<F>>,
        )>,
    >,
}

impl<'a, F: FieldExt> ConstraintBuilder<'a, F> {
    pub(super) fn new(meta: &'a mut ConstraintSystem<F>) -> Self {
        Self {
            meta,
            constraints: vec![],
            lookups: BTreeMap::new(),
        }
    }

    pub(super) fn finalize(self, enable: impl Fn(&mut VirtualCells<F>) -> Expression<F>) {
        for (name, builder) in self.constraints {
            self.meta.create_gate(&name, |meta| {
                builder(meta)
                    .into_iter()
                    .map(|constraint| constraint * enable(meta))
                    .collect::<Vec<_>>()
            });
        }
    }
}
