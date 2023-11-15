use std::marker::PhantomData;

use halo2_proofs::arithmetic::FieldExt;
use halo2_proofs::plonk::ConstraintSystem;
use halo2_proofs::plonk::Expression;
use halo2_proofs::plonk::VirtualCells;
use specs::encode::image_table::ImageTableEncoder;

use super::ImageTableConfig;

impl<F: FieldExt> ImageTableConfig<F> {
    fn expr(&self, meta: &mut VirtualCells<F>) -> Expression<F> {
        cfg_if::cfg_if! {
            if #[cfg(feature="uniform-circuit")] {
                use crate::curr;

                curr!(meta, self.col)
            } else {
                use crate::fixed_curr;

                fixed_curr!(meta, self.col)
            }
        }
    }

    pub(in crate::circuits) fn configure(meta: &mut ConstraintSystem<F>) -> Self {
        cfg_if::cfg_if! {
            if #[cfg(feature="uniform-circuit")] {
                let col = meta.named_advice_column(super::IMAGE_COL_NAME.to_owned());
            } else {
                let col = meta.fixed_column();
            }
        }
        meta.enable_equality(col);
        Self {
            col,
            _mark: PhantomData,
        }
    }

    pub fn instruction_lookup(
        &self,
        meta: &mut ConstraintSystem<F>,
        key: &'static str,
        expr: impl FnOnce(&mut VirtualCells<'_, F>) -> Expression<F>,
    ) {
        meta.lookup_any(key, |meta| {
            vec![(
                ImageTableEncoder::Instruction.encode(expr(meta)),
                self.expr(meta),
            )]
        });
    }

    pub fn init_memory_lookup(
        &self,
        meta: &mut ConstraintSystem<F>,
        key: &'static str,
        expr: impl FnOnce(&mut VirtualCells<'_, F>) -> Expression<F>,
    ) {
        meta.lookup_any(key, |meta| {
            vec![(
                ImageTableEncoder::InitMemory.encode(expr(meta)),
                self.expr(meta),
            )]
        });
    }

    pub fn br_table_lookup(
        &self,
        meta: &mut ConstraintSystem<F>,
        key: &'static str,
        expr: impl FnOnce(&mut VirtualCells<'_, F>) -> Expression<F>,
    ) {
        meta.lookup_any(key, |meta| {
            vec![(
                ImageTableEncoder::BrTable.encode(expr(meta)),
                self.expr(meta),
            )]
        });
    }
}
