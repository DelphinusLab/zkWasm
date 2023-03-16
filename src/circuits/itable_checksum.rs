use super::utils::bn_to_field;
use crate::circuits::config::max_itable_rows;
use crate::curr;
use halo2_proofs::arithmetic::FieldExt;
use halo2_proofs::circuit::AssignedCell;
use halo2_proofs::circuit::Layouter;
use halo2_proofs::plonk::Advice;
use halo2_proofs::plonk::Column;
use halo2_proofs::plonk::ConstraintSystem;
use halo2_proofs::plonk::Error;
use halo2_proofs::plonk::Expression;
use halo2_proofs::plonk::VirtualCells;
use specs::itable::InstructionTable;
use std::marker::PhantomData;

#[derive(Clone)]
pub struct InstructionTableConfig<F: FieldExt> {
    col: Column<Advice>,
    _mark: PhantomData<F>,
}

impl<F: FieldExt> InstructionTableConfig<F> {
    pub(in crate::circuits) fn configure(meta: &mut ConstraintSystem<F>) -> Self {
        Self {
            col: meta.advice_column(),
            _mark: PhantomData,
        }
    }

    pub fn configure_in_table(
        &self,
        meta: &mut ConstraintSystem<F>,
        key: &'static str,
        expr: impl FnOnce(&mut VirtualCells<'_, F>) -> Expression<F>,
    ) {
        meta.lookup_any(key, |meta| vec![(expr(meta), curr!(meta, self.col))]);
    }
}

#[derive(Clone)]
pub struct InstructionTableChip<F: FieldExt> {
    config: InstructionTableConfig<F>,
}

impl<F: FieldExt> InstructionTableChip<F> {
    pub fn new(config: InstructionTableConfig<F>) -> Self {
        InstructionTableChip { config }
    }

    pub fn assign(
        self,
        layouter: &mut impl Layouter<F>,
        instructions: &InstructionTable,
    ) -> Result<Vec<AssignedCell<F, F>>, Error> {
        let mut ret = vec![];

        layouter.assign_region(
            || "instruction table",
            |mut table| {
                let mut offset = 0;
                for v in instructions.entries().iter() {
                    let cell = table.assign_advice(
                        || "instruction table",
                        self.config.col,
                        offset,
                        || Ok(bn_to_field::<F>(&v.encode())),
                    )?;

                    ret.push(cell);
                    offset += 1;
                }

                let max_rows = max_itable_rows() as usize;
                assert!(offset < max_rows);

                while offset < max_rows {
                    let cell = table.assign_advice(
                        || "br table padding",
                        self.config.col,
                        offset,
                        || Ok(F::zero()),
                    )?;

                    ret.push(cell);
                    offset += 1;
                }

                Ok(())
            },
        )?;
        Ok(ret)
    }
}
