use std::collections::BTreeMap;

use halo2_proofs::{
    arithmetic::FieldExt,
    plonk::{ConstraintSystem, Expression, VirtualCells},
};

pub(crate) struct ConstraintBuilder<'a, F: FieldExt> {
    meta: &'a mut ConstraintSystem<F>,
    pub(crate) constraints: Vec<(
        &'static str,
        Box<dyn FnOnce(&mut VirtualCells<F>) -> Vec<Expression<F>>>,
    )>,
    pub(crate) lookups: BTreeMap<
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

    pub(crate) fn push(
        &mut self,
        name: &'static str,
        constraint: Box<dyn FnOnce(&mut VirtualCells<F>) -> Vec<Expression<F>>>,
    ) {
        self.constraints.push((name, constraint))
    }

    pub(crate) fn lookup(
        &mut self,
        foreign_table_id: &'static str,
        name: &'static str,
        builder: Box<dyn Fn(&mut VirtualCells<F>) -> Expression<F>>,
    ) {
        match self.lookups.get_mut(&foreign_table_id) {
            Some(lookups) => lookups.push((name, builder)),
            None => {
                self.lookups.insert(foreign_table_id, vec![(name, builder)]);
            }
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
