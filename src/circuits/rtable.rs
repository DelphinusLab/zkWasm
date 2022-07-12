use halo2_proofs::arithmetic::FieldExt;
use halo2_proofs::circuit::Layouter;
use halo2_proofs::plonk::ConstraintSystem;
use halo2_proofs::plonk::Error;
use halo2_proofs::plonk::Expression;
use halo2_proofs::plonk::TableColumn;
use halo2_proofs::plonk::VirtualCells;
use std::marker::PhantomData;

pub struct RangeTableConfig<F: FieldExt> {
    cols: [TableColumn; 1],
    _mark: PhantomData<F>,
}

impl<F: FieldExt> RangeTableConfig<F> {
    pub fn configure(cols: [TableColumn; 1]) -> Self {
        RangeTableConfig {
            cols,
            _mark: PhantomData,
        }
    }

    pub fn configure_in_range(
        &self,
        meta: &mut ConstraintSystem<F>,
        key: &'static str,
        expr: impl FnOnce(&mut VirtualCells<'_, F>) -> Expression<F>,
    ) {
        meta.lookup(key, |meta| vec![(expr(meta), self.cols[0])]);
    }
}

pub struct RangeTableChip<F: FieldExt> {
    config: RangeTableConfig<F>,
    _phantom: PhantomData<F>,
}

impl<F: FieldExt> RangeTableChip<F> {
    pub fn new(config: RangeTableConfig<F>) -> Self {
        RangeTableChip {
            config,
            _phantom: PhantomData,
        }
    }

    pub fn init(&self, layouter: &mut impl Layouter<F>, range: usize) -> Result<(), Error> {
        layouter.assign_table(
            || "common range table",
            |mut table| {
                for i in 0..range {
                    table.assign_cell(
                        || "range table",
                        self.config.cols[0],
                        i,
                        || Ok(F::from(i as u64)),
                    )?;
                }
                Ok(())
            },
        )?;
        Ok(())
    }
}
