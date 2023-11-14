use std::marker::PhantomData;

use halo2_proofs::arithmetic::FieldExt;
use halo2_proofs::plonk::Column;
use halo2_proofs::plonk::ConstraintSystem;
use halo2_proofs::plonk::Expression;
use halo2_proofs::plonk::Fixed;
use halo2_proofs::plonk::VirtualCells;
use specs::encode::image_table::ImageTableEncoder;

use super::ImageTableConfig;
use super::IMAGE_COL_NAME;
use crate::curr;

impl<F: FieldExt> ImageTableConfig<F> {
    pub(in crate::circuits) fn configure(
        meta: &mut ConstraintSystem<F>,
        _memory_addr_sel: Column<Fixed>,
    ) -> Self {
        let col = meta.named_advice_column(IMAGE_COL_NAME.to_owned());
        meta.enable_equality(col);
        Self {
            _memory_addr_sel,
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
                curr!(meta, self.col),
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
                (addr, fixed_curr!(meta, self._memory_addr_sel)),
                (
                    ImageTableEncoder::InitMemory.encode(encode),
                    curr!(meta, self.col),
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
                curr!(meta, self.col),
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
                curr!(meta, self.col),
            )]
        });
    }
}
