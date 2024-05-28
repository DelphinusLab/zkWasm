use std::collections::HashSet;
use std::marker::PhantomData;

use halo2_proofs::arithmetic::FieldExt;
use halo2_proofs::circuit::AssignedCell;
use halo2_proofs::circuit::Layouter;
use halo2_proofs::plonk::Column;
use halo2_proofs::plonk::ConstraintSystem;
use halo2_proofs::plonk::Error;
use halo2_proofs::plonk::Fixed;
use specs::mtable::LocationType;

use crate::circuits::image_table::ImageTableConfig;
use crate::circuits::mtable::MemoryTableConfig;
use crate::circuits::utils::image_table::ImageTableAssigner;
use crate::circuits::utils::image_table::ImageTableLayouter;

use super::jtable::JumpTableConfig;

#[derive(Clone)]
pub(in crate::circuits) struct PostImageTableConfig<F: FieldExt> {
    _mark: PhantomData<F>,
}

impl<F: FieldExt> PostImageTableConfig<F> {
    pub(in crate::circuits) fn configure(
        _meta: &mut ConstraintSystem<F>,
        _memory_addr_sel: Option<Column<Fixed>>,
        _memory_table: &MemoryTableConfig<F>,
        _frame_table: &JumpTableConfig<F>,
        _pre_image_table: &ImageTableConfig<F>,
    ) -> Self {
        Self { _mark: PhantomData }
    }
}

pub(in crate::circuits) struct PostImageTableChip<F: FieldExt> {
    _mark: PhantomData<F>,
}

impl<F: FieldExt> PostImageTableChip<F> {
    pub(in crate::circuits) fn new(_config: PostImageTableConfig<F>) -> Self {
        Self { _mark: PhantomData }
    }

    pub(in crate::circuits) fn assign(
        self,
        _layouter: impl Layouter<F>,
        _image_table_assigner: &ImageTableAssigner,
        _post_image_table: ImageTableLayouter<F>,
        _rest_memory_finalized_count: u32,
        _memory_finalized_set: HashSet<(LocationType, u32)>,
    ) -> Result<Option<(ImageTableLayouter<AssignedCell<F, F>>, AssignedCell<F, F>)>, Error> {
        Ok(None)
    }
}
