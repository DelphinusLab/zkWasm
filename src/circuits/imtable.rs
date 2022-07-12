use super::utils::bn_to_field;
use crate::spec::imtable::InitMemoryTableEntry;
use halo2_proofs::{
    arithmetic::FieldExt,
    circuit::Layouter,
    plonk::{ConstraintSystem, Error, Expression, TableColumn, VirtualCells},
};
use num_bigint::BigUint;
use num_traits::{One, Zero};
use std::marker::PhantomData;

impl InitMemoryTableEntry {
    pub fn encode(&self) -> BigUint {
        let mut bn = BigUint::zero();
        bn += self.mmid;
        bn <<= 16;
        bn += self.offset;
        bn <<= 64;
        bn += self.value;
        bn
    }
}

pub const MINIT_TABLE_COLUMNS: usize = 3usize;

pub struct MInitTableConfig<F: FieldExt> {
    col: TableColumn,
    _mark: PhantomData<F>,
}

impl<F: FieldExt> MInitTableConfig<F> {
    pub fn new(col: TableColumn) -> Self {
        Self {
            col,
            _mark: PhantomData,
        }
    }

    pub fn encode(
        &self,
        mmid: Expression<F>,
        offset: Expression<F>,
        value: Expression<F>,
    ) -> Expression<F> {
        mmid * Expression::Constant(bn_to_field(&(BigUint::one() << 80)))
            + offset * Expression::Constant(bn_to_field(&(BigUint::one() << 64)))
            + value
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

pub struct MInitTableChip<F: FieldExt> {
    config: MInitTableConfig<F>,
    _phantom: PhantomData<F>,
}

impl<F: FieldExt> MInitTableChip<F> {
    pub fn add_memory_init(
        self,
        layouter: &mut impl Layouter<F>,
        minit: Vec<InitMemoryTableEntry>,
    ) -> Result<(), Error> {
        layouter.assign_table(
            || "minit",
            |mut table| {
                for (i, v) in minit.iter().enumerate() {
                    table.assign_cell(
                        || "minit talbe",
                        self.config.col,
                        i,
                        || Ok(bn_to_field::<F>(&v.encode())),
                    )?;
                }
                Ok(())
            },
        )?;
        Ok(())
    }
}
