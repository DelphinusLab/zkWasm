use super::{config::IMTABLE_COLOMNS, utils::bn_to_field, Encode};
use halo2_proofs::{
    arithmetic::FieldExt,
    circuit::Layouter,
    plonk::{ConstraintSystem, Error, Expression, TableColumn, VirtualCells},
};
use num_bigint::BigUint;
use num_traits::{One, Zero};
use specs::imtable::InitMemoryTableEntry;
use std::marker::PhantomData;

impl Encode for InitMemoryTableEntry {
    fn encode(&self) -> BigUint {
        let mut bn = BigUint::zero();
        bn += self.ltype as u64;
        bn <<= 16;
        bn += if self.is_mutable { 1u64 } else { 0u64 };
        bn <<= 16;
        bn += self.mmid;
        bn <<= 16;
        bn += self.offset;
        bn <<= 64;
        bn += self.value;
        bn
    }
}

#[derive(Clone)]
pub struct InitMemoryTableConfig<F: FieldExt> {
    col: [TableColumn; IMTABLE_COLOMNS],
    _mark: PhantomData<F>,
}

impl<F: FieldExt> InitMemoryTableConfig<F> {
    pub fn configure(col: [TableColumn; IMTABLE_COLOMNS]) -> Self {
        Self {
            col,
            _mark: PhantomData,
        }
    }

    pub fn encode(
        &self,
        is_mutable: Expression<F>,
        ltype: Expression<F>,
        mmid: Expression<F>,
        offset: Expression<F>,
        value: Expression<F>,
    ) -> Expression<F> {
        ltype * Expression::Constant(bn_to_field(&(BigUint::one() << 112)))
            + is_mutable * Expression::Constant(bn_to_field(&(BigUint::one() << 96)))
            + mmid * Expression::Constant(bn_to_field(&(BigUint::one() << 80)))
            + offset * Expression::Constant(bn_to_field(&(BigUint::one() << 64)))
            + value
    }

    pub fn configure_in_table(
        &self,
        meta: &mut ConstraintSystem<F>,
        key: &'static str,
        expr: impl FnOnce(&mut VirtualCells<'_, F>) -> Expression<F>,
        index: usize,
    ) {
        meta.lookup(key, |meta| vec![(expr(meta), self.col[index])]);
    }
}

pub struct MInitTableChip<F: FieldExt> {
    config: InitMemoryTableConfig<F>,
}

impl<F: FieldExt> MInitTableChip<F> {
    pub fn new(config: InitMemoryTableConfig<F>) -> Self {
        MInitTableChip { config }
    }

    pub fn assign(
        self,
        layouter: &mut impl Layouter<F>,
        minit: &Vec<InitMemoryTableEntry>,
    ) -> Result<(), Error> {
        layouter.assign_table(
            || "minit",
            |mut table| {
                for i in 0..IMTABLE_COLOMNS {
                    table.assign_cell(|| "minit table", self.config.col[i], 0, || Ok(F::zero()))?;
                }

                for v in minit.iter() {
                    table.assign_cell(
                        || "minit table",
                        self.config.col[v.offset as usize % IMTABLE_COLOMNS],
                        v.offset as usize / IMTABLE_COLOMNS + 1,
                        || Ok(bn_to_field::<F>(&v.encode())),
                    )?;
                }
                Ok(())
            },
        )?;
        Ok(())
    }
}
