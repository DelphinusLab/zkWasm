use super::utils::bn_to_field;
use halo2_proofs::arithmetic::FieldExt;
use halo2_proofs::circuit::Layouter;
use halo2_proofs::plonk::ConstraintSystem;
use halo2_proofs::plonk::Error;
use halo2_proofs::plonk::Expression;
use halo2_proofs::plonk::TableColumn;
use halo2_proofs::plonk::VirtualCells;
use specs::encode::init_memory_table::encode_init_memory_table_entry;
use specs::imtable::InitMemoryTable;
use specs::mtable::LocationType;
use std::marker::PhantomData;

#[derive(Clone)]
pub struct InitMemoryTableConfig<F: FieldExt> {
    col: TableColumn,
    _mark: PhantomData<F>,
}

impl<F: FieldExt> InitMemoryTableConfig<F> {
    pub(in crate::circuits) fn configure(meta: &mut ConstraintSystem<F>) -> Self {
        Self {
            col: meta.lookup_table_column(),
            _mark: PhantomData,
        }
    }

    pub fn encode(
        &self,
        is_mutable: Expression<F>,
        ltype: Expression<F>,
        start_offset: Expression<F>,
        end_offset: Expression<F>,
        value: Expression<F>,
    ) -> Expression<F> {
        encode_init_memory_table_entry(ltype, is_mutable, start_offset, end_offset, value)
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

pub struct InitMemoryTableChip<F: FieldExt> {
    config: InitMemoryTableConfig<F>,
}

impl<F: FieldExt> InitMemoryTableChip<F> {
    pub fn new(config: InitMemoryTableConfig<F>) -> Self {
        InitMemoryTableChip { config }
    }

    pub fn assign(
        self,
        layouter: &mut impl Layouter<F>,
        init_memory_entries: &InitMemoryTable,
    ) -> Result<(), Error> {
        layouter.assign_table(
            || "init memory table",
            |mut table| {
                table.assign_cell(
                    || "init memory table empty",
                    self.config.col,
                    0,
                    || Ok(F::zero()),
                )?;

                let heap_entries = init_memory_entries.filter(LocationType::Heap);
                let global_entries = init_memory_entries.filter(LocationType::Global);

                let mut idx = 0;

                for v in heap_entries.into_iter().chain(global_entries.into_iter()) {
                    table.assign_cell(
                        || "init memory table cell",
                        self.config.col,
                        idx + 1,
                        || Ok(bn_to_field::<F>(&v.encode())),
                    )?;

                    idx += 1;
                }

                Ok(())
            },
        )?;
        Ok(())
    }
}
