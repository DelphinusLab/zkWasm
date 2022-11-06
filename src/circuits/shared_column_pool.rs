use std::marker::PhantomData;

use crate::{
    circuits::config::{ETABLE_END_OFFSET, ETABLE_START_OFFSET},
    curr, fixed_curr,
};
use halo2_proofs::{
    arithmetic::FieldExt,
    circuit::{Layouter, Region},
    plonk::{Advice, Column, ConstraintSystem, Error, Expression, Fixed, VirtualCells},
};

use super::{
    config::{
        FOREIGN_HELPER_END_OFFSET, FOREIGN_HELPER_START_OFFSET, MTABLE_END_OFFSET,
        MTABLE_START_OFFSET,
    },
    rtable::{RangeTableConfig, RangeTableMixColumn},
};

#[derive(Clone, Copy)]
pub struct DynTableLookupColumn<F> {
    pub internal: Column<Advice>,
    lookup: Column<Fixed>,
    _mark: PhantomData<F>,
}

impl<F: FieldExt> DynTableLookupColumn<F> {
    pub fn expr(&self, meta: &mut VirtualCells<'_, F>) -> Expression<F> {
        curr!(meta, self.internal)
    }
    pub fn assign_lookup<'a>(
        &self,
        region: &mut Region<'a, F>,
        offset: usize,
        kind: RangeTableMixColumn,
    ) -> Result<(), Error> {
        region.assign_fixed(
            || "DynTableLookupColumn lookup",
            self.lookup,
            offset,
            || Ok(F::from(kind as u64)),
        )?;

        Ok(())
    }
}

//const U8_COLUMNS: usize = 2;
//const U4_COLUMNS: usize = 5;
//const U16_COLUMNS: usize = 1;
const EXTRA_ADVICES: usize = 6;
const DYN_COLUMNS: usize = 7;

#[derive(Clone)]
pub struct SharedColumnPool<F> {
    sel: Column<Fixed>,
    advices: [Column<Advice>; EXTRA_ADVICES],
    dyn_cols: [DynTableLookupColumn<F>; DYN_COLUMNS],
    _mark: PhantomData<F>,
}

impl<F: FieldExt> SharedColumnPool<F> {
    pub fn configure(meta: &mut ConstraintSystem<F>, rtable: &RangeTableConfig<F>) -> Self {
        let sel = meta.fixed_column();
        let advices = [(); EXTRA_ADVICES].map(|_| meta.advice_column());
        let dyn_cols = [(); DYN_COLUMNS].map(|_| DynTableLookupColumn::<F> {
            internal: meta.advice_column(),
            lookup: meta.fixed_column(),
            _mark: PhantomData,
        });

        for i in 0..DYN_COLUMNS {
            meta.lookup("dyn lookup", |meta| {
                let x = fixed_curr!(meta, dyn_cols[i].lookup);

                let prefix = RangeTableMixColumn::U4.largrange(x.clone())
                    * RangeTableMixColumn::U4.prefix::<F>()
                    + RangeTableMixColumn::U8.largrange(x.clone())
                        * RangeTableMixColumn::U8.prefix::<F>()
                    + RangeTableMixColumn::U16.largrange(x.clone())
                        * RangeTableMixColumn::U16.prefix::<F>()
                    + RangeTableMixColumn::Pow.largrange(x.clone())
                        * RangeTableMixColumn::Pow.prefix::<F>()
                    + RangeTableMixColumn::OffsetLenBits.largrange(x.clone())
                        * RangeTableMixColumn::OffsetLenBits.prefix::<F>();

                vec![(
                    x * (prefix + curr!(meta, dyn_cols[i].internal)),
                    rtable.mix_col,
                )]
            });
        }

        SharedColumnPool::<F> {
            sel,
            //u8_col,
            //u4_cols,
            //u16_cols,
            advices,
            dyn_cols,
            _mark: PhantomData,
        }
    }

    pub fn acquire_sel_col(&self) -> Column<Fixed> {
        self.sel.clone()
    }

    pub fn dyn_col_iter(&self) -> impl Iterator<Item = DynTableLookupColumn<F>> {
        self.dyn_cols.into_iter()
    }

    pub fn advice_iter(&self) -> impl Iterator<Item = Column<Advice>> {
        self.advices.into_iter()
    }
}

pub struct SharedColumnChip<F> {
    config: SharedColumnPool<F>,
}

impl<F: FieldExt> SharedColumnChip<F> {
    pub fn new(config: SharedColumnPool<F>) -> Self {
        SharedColumnChip::<F> { config }
    }

    pub fn init(&self, layouter: &mut impl Layouter<F>) -> Result<(), Error> {
        layouter.assign_region(
            || "shared column",
            |mut region| {
                for o in FOREIGN_HELPER_START_OFFSET..FOREIGN_HELPER_END_OFFSET {
                    region.assign_fixed(
                        || "shared column sel",
                        self.config.sel,
                        o,
                        || Ok(F::from(1u64)),
                    )?;
                }

                for o in ETABLE_START_OFFSET..ETABLE_END_OFFSET {
                    region.assign_fixed(
                        || "shared column sel",
                        self.config.sel,
                        o,
                        || Ok(F::from(1u64)),
                    )?;
                }

                for o in MTABLE_START_OFFSET..MTABLE_END_OFFSET {
                    region.assign_fixed(
                        || "shared column sel",
                        self.config.sel,
                        o,
                        || Ok(F::from(1u64)),
                    )?;
                }

                Ok(())
            },
        )?;

        Ok(())
    }
}
