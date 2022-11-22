use super::{config::IMTABLE_COLOMNS, utils::bn_to_field};
use halo2_proofs::{
    arithmetic::FieldExt,
    circuit::Layouter,
    plonk::{ConstraintSystem, Error, Expression, TableColumn, VirtualCells},
};
use num_bigint::BigUint;
use num_traits::One;
use specs::{
    encode::FromBn,
    imtable::{ImportMemoryEntry, InitMemoryEntry, InitMemoryTable, InitMemoryTableEntry},
};
use std::{
    marker::PhantomData,
    ops::{Add, Mul},
};

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
        minit: &InitMemoryTable,
    ) -> Result<(), Error> {
        layouter.assign_table(
            || "minit",
            |mut table| {
                for i in 0..IMTABLE_COLOMNS {
                    table.assign_cell(|| "minit table", self.config.col[i], 0, || Ok(F::zero()))?;
                }

                let import_entries = minit.filter_import();
                let heap_entries = minit.filter_memory_init();
                let mut init_entries = minit.filter_global_init();

                init_entries.push(heap_entries);

                let mut offset = 1;

                {
                    for group in init_entries.iter() {
                        let mut idx = 0;

                        for e in group {
                            table.assign_cell(
                                || "minit table",
                                self.config.col[e.offset as usize % IMTABLE_COLOMNS],
                                offset,
                                || Ok(bn_to_field::<F>(&e.encode())),
                            )?;

                            idx += 1;

                            if idx == IMTABLE_COLOMNS {
                                idx = 0;
                                offset += 1;
                            }
                        }

                        /*
                         * Fill blank cells in the last row to make halo2 happy.
                         */
                        if idx % IMTABLE_COLOMNS != 0 {
                            for blank_col in idx..IMTABLE_COLOMNS {
                                table.assign_cell(
                                    || "minit table",
                                    self.config.col[blank_col],
                                    offset,
                                    || Ok(F::zero()),
                                )?;
                            }

                            offset += 1;
                        }
                    }
                }

                {
                    // Import table
                    for e in import_entries.into_iter() {
                        table.assign_cell(
                            || "import table",
                            self.config.col[0],
                            offset,
                            || Ok(bn_to_field::<F>(&e.encode())),
                        )?;

                        for i in 1..IMTABLE_COLOMNS {
                            table.assign_cell(
                                || "import table",
                                self.config.col[i],
                                offset,
                                || Ok(F::zero()),
                            )?;
                        }

                        offset += 1;
                    }
                }

                Ok(())
            },
        )?;
        Ok(())
    }
}

pub struct IMTableEncode;

impl IMTableEncode {
    pub fn encode_for_init<T: FromBn + Add<T, Output = T> + Mul<T, Output = T>>(
        is_mutable: T,
        ltype: T,
        mmid: T,
        offset: T,
        value: T,
    ) -> T {
        T::from_bn(&(BigUint::from(1u64))) * T::from_bn(&(BigUint::one() << 128))
            + ltype * T::from_bn(&(BigUint::one() << 112))
            + is_mutable * T::from_bn(&(BigUint::one() << 96))
            + mmid * T::from_bn(&(BigUint::one() << 80))
            + offset * T::from_bn(&(BigUint::one() << 64))
            + value
    }

    pub(crate) fn encode_for_import<T: FromBn + Add<T, Output = T> + Mul<T, Output = T>>(
        ltype: T,
        origin_moid: T,
        origin_idx: T,
        moid: T,
        idx: T,
    ) -> T {
        T::from_bn(&(BigUint::from(2u64))) * T::from_bn(&(BigUint::one() << 128))
            + ltype * T::from_bn(&(BigUint::one() << 112))
            + origin_moid * T::from_bn(&(BigUint::one() << 96))
            + origin_idx * T::from_bn(&(BigUint::one() << 80))
            + moid * T::from_bn(&(BigUint::one() << 64))
            + idx
    }
}

pub trait EncodeImTableEntry {
    fn encode(&self) -> BigUint;
}

impl EncodeImTableEntry for ImportMemoryEntry {
    fn encode(&self) -> BigUint {
        IMTableEncode::encode_for_import(
            BigUint::from(self.ltype as u64),
            BigUint::from(self.origin_moid),
            BigUint::from(self.origin_idx),
            BigUint::from(self.moid),
            BigUint::from(self.idx),
        )
    }
}

impl EncodeImTableEntry for InitMemoryEntry {
    fn encode(&self) -> BigUint {
        IMTableEncode::encode_for_init(
            BigUint::from(self.is_mutable as u64),
            BigUint::from(self.ltype as u64),
            BigUint::from(self.mmid),
            BigUint::from(self.offset),
            BigUint::from(self.value),
        )
    }
}

impl EncodeImTableEntry for InitMemoryTableEntry {
    fn encode(&self) -> BigUint {
        match self {
            InitMemoryTableEntry::Import(e) => e.encode(),
            InitMemoryTableEntry::Init(e) => e.encode(),
        }
    }
}
