use super::{config::IMTABLE_COLUMNS, utils::bn_to_field};
use halo2_proofs::{
    arithmetic::FieldExt,
    circuit::Layouter,
    plonk::{ConstraintSystem, Error, Expression, TableColumn, VirtualCells},
};
use specs::{
    encode::init_memory_table::encode_init_memory_table_entry, imtable::InitMemoryTable,
    mtable::LocationType,
};
use std::marker::PhantomData;

#[derive(Clone)]
pub struct InitMemoryTableConfig<F: FieldExt> {
    col: [TableColumn; IMTABLE_COLUMNS],
    _mark: PhantomData<F>,
}

impl<F: FieldExt> InitMemoryTableConfig<F> {
    pub fn configure(col: [TableColumn; IMTABLE_COLUMNS]) -> Self {
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
        encode_init_memory_table_entry(ltype, is_mutable, offset, value)
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
                for i in 0..IMTABLE_COLUMNS {
                    table.assign_cell(|| "minit table", self.config.col[i], 0, || Ok(F::zero()))?;
                }

                let heap_entries = minit.filter(LocationType::Heap);
                let global_entries = minit.filter(LocationType::Global);

                /*
                 * Since the number of heap entries is always n * PAGE_SIZE / sizeof(u64).
                 */
                assert_eq!(heap_entries.len() % IMTABLE_COLUMNS, 0);

                let mut idx = 0;

                for v in heap_entries.into_iter().chain(global_entries.into_iter()) {
                    table.assign_cell(
                        || "minit table",
                        self.config.col[idx % IMTABLE_COLUMNS],
                        idx / IMTABLE_COLUMNS + 1,
                        || Ok(bn_to_field::<F>(&v.encode())),
                    )?;

                    idx += 1;
                }

                /*
                 * Fill blank cells in the last row to make halo2 happy.
                 */
                if idx % IMTABLE_COLUMNS != 0 {
                    for blank_col in (idx % IMTABLE_COLUMNS)..IMTABLE_COLUMNS {
                        table.assign_cell(
                            || "minit table",
                            self.config.col[blank_col],
                            idx / IMTABLE_COLUMNS + 1,
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
