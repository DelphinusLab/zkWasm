use crate::circuits::config::max_imtable_rows;
use crate::curr;

use super::utils::bn_to_field;
use halo2_proofs::arithmetic::FieldExt;
use halo2_proofs::circuit::Layouter;
use halo2_proofs::plonk::Advice;
use halo2_proofs::plonk::Column;
use halo2_proofs::plonk::ConstraintSystem;
use halo2_proofs::plonk::Error;
use halo2_proofs::plonk::Expression;
use halo2_proofs::plonk::VirtualCells;
use specs::encode::init_memory_table::encode_init_memory_table_entry;
use specs::imtable::InitMemoryTable;
use specs::mtable::LocationType;
use std::marker::PhantomData;

#[derive(Clone)]
pub struct InitMemoryTableConfig<F: FieldExt> {
    col: Column<Advice>,
    _mark: PhantomData<F>,
}

impl<F: FieldExt> InitMemoryTableConfig<F> {
    pub(in crate::circuits) fn configure(meta: &mut ConstraintSystem<F>) -> Self {
        Self {
            col: meta.advice_column(),
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
    ) {
        meta.lookup_any(key, |meta| vec![(expr(meta), curr!(meta, self.col))]);
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
        let mut ret = vec![];

        layouter.assign_region(
            || "init memory table",
            |mut table| {
                let heap_entries = init_memory_entries.filter(LocationType::Heap);
                let global_entries = init_memory_entries.filter(LocationType::Global);

                let mut offset = 0;

                for v in heap_entries.into_iter().chain(global_entries.into_iter()) {
                    let cell = table.assign_advice(
                        || "init memory table cell",
                        self.config.col,
                        offset,
                        || Ok(bn_to_field::<F>(&v.encode())),
                    )?;

                    ret.push(cell);
                    offset += 1;
                }

                let max_rows = max_imtable_rows() as usize;
                assert!(offset < max_rows);

                while offset < max_imtable_rows() as usize {
                    let cell = table.assign_advice(
                        || "init memory table padding",
                        self.config.col,
                        offset,
                        || Ok(F::zero()),
                    )?;

                    ret.push(cell);
                    offset += 1;
                }

                Ok(())
            },
        )?;
        Ok(())
    }
}
