use super::{config::IMTABLE_COLOMNS, utils::bn_to_field, Encode};
use halo2_proofs::{
    arithmetic::FieldExt,
    circuit::Layouter,
    plonk::{ConstraintSystem, Error, Expression, TableColumn, VirtualCells},
};
use num_bigint::BigUint;
use num_traits::{One, Zero};
use specs::{
    imtable::{InitMemoryTable, InitMemoryTableEntry},
    mtable::LocationType,
};
use std::marker::PhantomData;

impl Encode for InitMemoryTableEntry {
    fn encode(&self) -> BigUint {
        let mut bn = BigUint::zero();
        bn += self.ltype as u64;
        bn <<= 16;
        bn += if self.is_mutable { 1u64 } else { 0u64 };
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
        offset: Expression<F>,
        value: Expression<F>,
    ) -> Expression<F> {
        ltype * Expression::Constant(bn_to_field(&(BigUint::one() << 96)))
            + is_mutable * Expression::Constant(bn_to_field(&(BigUint::one() << 80)))
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
        minit: &InitMemoryTable,
    ) -> Result<(), Error> {
        layouter.assign_table(
            || "minit",
            |mut table| {
                for i in 0..IMTABLE_COLOMNS {
                    table.assign_cell(|| "minit table", self.config.col[i], 0, || Ok(F::zero()))?;
                }

                let heap_entries = minit.filter(LocationType::Heap);
                let global_entries = minit.filter(LocationType::Global);

                /*
                 * Since the number of heap entries is always n * PAGE_SIZE / sizeof(u64).
                 */
                assert_eq!(heap_entries.len() % IMTABLE_COLOMNS, 0);

                let mut idx = 0;

                for v in heap_entries.into_iter().chain(global_entries.into_iter()) {
                    table.assign_cell(
                        || "minit table",
                        self.config.col[idx % IMTABLE_COLOMNS],
                        idx / IMTABLE_COLOMNS + 1,
                        || Ok(bn_to_field::<F>(&v.encode())),
                    )?;

                    idx += 1;
                }

                /*
                 * Fill blank cells in the last row to make halo2 happy.
                 */
                if idx % IMTABLE_COLOMNS != 0 {
                    for blank_col in (idx % IMTABLE_COLOMNS)..IMTABLE_COLOMNS {
                        table.assign_cell(
                            || "minit table",
                            self.config.col[blank_col],
                            idx / IMTABLE_COLOMNS + 1,
                            || Ok(F::zero()),
                        )?;
                    }
                }

                Ok(())
            },
        )?;
        Ok(())
    }
}
