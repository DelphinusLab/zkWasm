use std::marker::PhantomData;

use halo2_proofs::arithmetic::FieldExt;
use halo2_proofs::plonk::Column;
use halo2_proofs::plonk::ConstraintSystem;
use halo2_proofs::plonk::Expression;
use halo2_proofs::plonk::Fixed;
use halo2_proofs::plonk::VirtualCells;
use specs::encode::image_table::ImageTableEncoder;

use super::ImageTableConfig;

impl<F: FieldExt> ImageTableConfig<F> {
    pub(in crate::circuits) fn configure(
        meta: &mut ConstraintSystem<F>,
        memory_addr_sel: Option<Column<Fixed>>,
    ) -> Self {
        cfg_if::cfg_if! {
            if #[cfg(feature="uniform-circuit")] {
                let col = meta.named_advice_column(super::IMAGE_COL_NAME.to_owned());

                if cfg!(feature="continuation") {
                }
            } else {
                let col = meta.fixed_column();
            }
        }

        meta.enable_equality(col);

        Self {
            memory_addr_sel,
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

    #[cfg(feature = "continuation")]
    pub fn init_memory_lookup(
        &self,
        meta: &mut ConstraintSystem<F>,
        key: &'static str,
        expr: impl FnOnce(&mut VirtualCells<'_, F>) -> (Expression<F>, Expression<F>),
    ) {
        use crate::fixed_curr;

        meta.lookup_any(key, |meta| {
            let (addr, encode) = expr(meta);

            vec![
                (addr, fixed_curr!(meta, self.memory_addr_sel.unwrap())),
                (
                    ImageTableEncoder::InitMemory.encode(encode),
                    self.expr(meta),
                ),
            ]
        });
    }

    #[cfg(not(feature = "continuation"))]
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
