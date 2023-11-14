use halo2_proofs::arithmetic::FieldExt;
use halo2_proofs::circuit::AssignedCell;
use halo2_proofs::circuit::Layouter;
use halo2_proofs::plonk::Column;
use halo2_proofs::plonk::ConstraintSystem;
use halo2_proofs::plonk::Error;
use halo2_proofs::plonk::Fixed;

use super::image_table::ImageTableConfig;
use super::image_table::ImageTableLayouter;
use super::mtable::MemoryTableConfig;

pub(self) mod continuation;
pub(self) mod trivial;

pub(in crate::circuits) trait PostImageTableConfigTrait<F: FieldExt> {
    fn configure(
        _meta: &mut ConstraintSystem<F>,
        _memory_addr_sel: Column<Fixed>,
        _memory_table: &MemoryTableConfig<F>,
        _pre_image_table: &ImageTableConfig<F>,
    ) -> Self;
}

pub(in crate::circuits) trait PostImageTableChipTrait<
    F: FieldExt,
    Config: PostImageTableConfigTrait<F>,
>
{
    fn new(config: Config, circuit_maximal_pages: u32) -> Self;
    fn assign(
        self,
        layouter: &mut impl Layouter<F>,
        pre_image_table: ImageTableLayouter<F>,
        post_image_table: ImageTableLayouter<F>,
        permutation_cells: ImageTableLayouter<AssignedCell<F, F>>,
    ) -> Result<(), Error>;
}

cfg_if::cfg_if! {
    if #[cfg(feature = "continuation")] {
        use self::continuation::*;

        pub(in crate::circuits) type PostImageTableConfig<F> = ContinuationPostImageTableConfig<F>;
        pub(in crate::circuits) type PostImageTableChip<F> = ContinuationPostImageTableChip<F>;

    } else {
        use self::trivial::*;

        pub(in crate::circuits) type PostImageTableConfig<F> = TrivialPostImageTableConfig<F>;
        pub(in crate::circuits) type PostImageTableChip<F> = TrivialPostImageTableChip<F>;
    }
}
