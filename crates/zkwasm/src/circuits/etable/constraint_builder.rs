use std::collections::BTreeMap;

use halo2_proofs::arithmetic::FieldExt;
use halo2_proofs::plonk::ConstraintSystem;
use halo2_proofs::plonk::Expression;
use halo2_proofs::plonk::VirtualCells;

use crate::foreign::ForeignTableConfig;

pub(crate) struct ConstraintBuilder<'a, 'b, F: FieldExt> {
    meta: &'a mut ConstraintSystem<F>,
    foreign_table_configs: &'b BTreeMap<&'static str, Box<dyn ForeignTableConfig<F>>>,
    pub(crate) constraints: Vec<(
        &'static str,
        Box<dyn FnOnce(&mut VirtualCells<F>) -> Vec<Expression<F>>>,
    )>,
    pub(crate) lookups: BTreeMap<
        &'static str,
        Vec<(
            &'static str,
            Box<dyn Fn(&mut VirtualCells<F>) -> Vec<Expression<F>>>,
        )>,
    >,
}

impl<'a, 'b, F: FieldExt> ConstraintBuilder<'a, 'b, F> {
    pub(super) fn new(
        meta: &'a mut ConstraintSystem<F>,
        foreign_table_configs: &'b BTreeMap<&'static str, Box<dyn ForeignTableConfig<F>>>,
    ) -> Self {
        Self {
            meta,
            foreign_table_configs,
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
        builder: Box<dyn Fn(&mut VirtualCells<F>) -> Vec<Expression<F>>>,
    ) {
        match self.lookups.get_mut(&foreign_table_id) {
            Some(lookups) => lookups.push((name, builder)),
            None => {
                self.lookups.insert(foreign_table_id, vec![(name, builder)]);
            }
        }
    }

    pub(super) fn finalize(
        self,
        selector: impl Fn(&mut VirtualCells<F>) -> (Expression<F>, Expression<F>),
    ) {
        for (name, builder) in self.constraints {
            self.meta.create_gate(name, |meta| {
                builder(meta)
                    .into_iter()
                    .map(|constraint| {
                        let (step_sel, op_sel) = selector(meta);

                        constraint * step_sel * op_sel
                    })
                    .collect::<Vec<_>>()
            });
        }

        for (id, lookups) in self.lookups {
            let config = self.foreign_table_configs.get(&id).unwrap();

            for (name, expr) in lookups {
                config.configure_in_table(self.meta, name, &|meta| {
                    expr(meta)
                        .into_iter()
                        .map(|expr| {
                            let (step_sel, _op_sel) = selector(meta);
                            expr * step_sel
                        })
                        .collect()
                });
            }
        }
    }
}
