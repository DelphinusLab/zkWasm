use super::utils::bn_to_field;
use halo2_proofs::arithmetic::FieldExt;
use halo2_proofs::circuit::Layouter;
use halo2_proofs::plonk::ConstraintSystem;
use halo2_proofs::plonk::Error;
use halo2_proofs::plonk::Expression;
use halo2_proofs::plonk::TableColumn;
use halo2_proofs::plonk::VirtualCells;
use specs::itable::InstructionTable;
use std::marker::PhantomData;

#[derive(Clone)]
pub struct InstructionTableConfig<F: FieldExt> {
    col: TableColumn,
    _mark: PhantomData<F>,
}

impl<F: FieldExt> InstructionTableConfig<F> {
    pub fn configure(col: TableColumn) -> Self {
        InstructionTableConfig {
            col,
            _mark: PhantomData,
        }
    }

    pub fn configure_in_table(
        &self,
        meta: &mut ConstraintSystem<F>,
        key: &'static str,
        expr: impl FnOnce(&mut VirtualCells<'_, F>) -> Expression<F>,
    ) {
        meta.lookup(key, |meta| vec![(expr(meta), self.col)]);
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
    ) -> Result<(), Error> {
        layouter.assign_table(
            || "itable",
            |mut table| {
                table.assign_cell(|| "inst_init table", self.config.col, 0, || Ok(F::zero()))?;
                for (i, v) in instructions.entries().iter().enumerate() {
                    table.assign_cell(
                        || "inst_init table",
                        self.config.col,
                        i + 1,
                        || Ok(bn_to_field::<F>(&v.encode())),
                    )?;
                }
                Ok(())
            },
        )?;
        Ok(())
    }
}
