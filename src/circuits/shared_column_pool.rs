use std::marker::PhantomData;

use halo2_proofs::{
    arithmetic::FieldExt,
    circuit::Layouter,
    plonk::{Advice, Column, ConstraintSystem, Error, Fixed},
};

use crate::{
    circuits::config::{ETABLE_END_OFFSET, ETABLE_START_OFFSET},
    curr, fixed_curr,
};

use super::{
    config::{
        FOREIGN_HELPER_END_OFFSET, FOREIGN_HELPER_START_OFFSET, MTABLE_END_OFFSET,
        MTABLE_START_OFFSET,
    },
    rtable::{RangeTableConfig, RangeTableMixColumn},
};

#[derive(Clone)]
pub struct DynTableLookupColumn {
    pub internal: Column<Advice>,
    pub lookup: Column<Fixed>,
}

const U8_COLUMNS: usize = 2;
const U4_COLUMNS: usize = 5;
const U16_COLUMNS: usize = 1;
const EXTRA_ADVICES: usize = 6;
const DYN_COLUMNS: usize = 1;

#[derive(Clone)]
pub struct SharedColumnPool<F> {
    sel: Column<Fixed>,
    u4_cols: [Column<Advice>; U4_COLUMNS],
    u8_col: [Column<Advice>; U8_COLUMNS],
    u16_cols: [Column<Advice>; U16_COLUMNS],
    advices: [Column<Advice>; EXTRA_ADVICES],
    dyn_cols: [DynTableLookupColumn; DYN_COLUMNS],
    _mark: PhantomData<F>,
}

impl<F: FieldExt> SharedColumnPool<F> {
    pub fn configure(meta: &mut ConstraintSystem<F>, rtable: &RangeTableConfig<F>) -> Self {
        let sel = meta.fixed_column();
        let u4_cols = [(); U4_COLUMNS].map(|_| meta.advice_column());
        let u8_col = [(); U8_COLUMNS].map(|_| meta.advice_column());
        let u16_cols = [(); U16_COLUMNS].map(|_| meta.advice_column());
        let advices = [(); EXTRA_ADVICES].map(|_| meta.advice_column());
        let dyn_cols = [(); DYN_COLUMNS].map(|_| DynTableLookupColumn {
            internal: meta.advice_column(),
            lookup: meta.fixed_column(),
        });

        for i in 0..U8_COLUMNS {
            rtable.configure_in_u8_range(meta, "shared column u8", |meta| {
                curr!(meta, u8_col[i]) * fixed_curr!(meta, sel)
            });
        }

        for i in 0..U4_COLUMNS {
            rtable.configure_in_u4_range(meta, &"shared column u4", |meta| {
                curr!(meta, u4_cols[i]) * fixed_curr!(meta, sel)
            });
        }

        for i in 0..U16_COLUMNS {
            rtable.configure_in_u16_range(meta, &"shared column u16", |meta| {
                curr!(meta, u16_cols[i]) * fixed_curr!(meta, sel)
            });
        }

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
            u8_col,
            u4_cols,
            u16_cols,
            advices,
            dyn_cols,
            _mark: PhantomData,
        }
    }

    pub fn acquire_sel_col(&self) -> Column<Fixed> {
        self.sel.clone()
    }

    pub fn acquire_u8_col(&self, index: usize) -> Column<Advice> {
        self.u8_col[index].clone()
    }

    pub fn acquire_u4_col(&self, index: usize) -> Column<Advice> {
        self.u4_cols[index].clone()
    }

    pub fn acquire_u16_col(&self, index: usize) -> Column<Advice> {
        self.u16_cols[index].clone()
    }

    pub fn acquire_dyn_col(&self, index: usize) -> DynTableLookupColumn {
        self.dyn_cols[index].clone()
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
