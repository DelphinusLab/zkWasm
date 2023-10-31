use halo2_proofs::arithmetic::FieldExt;
use halo2_proofs::circuit::Cell;
use halo2_proofs::circuit::Layouter;
use halo2_proofs::plonk::ConstraintSystem;
use halo2_proofs::plonk::Error;

use super::image_table::ImageTableLayouter;

pub(self) mod continuation;
pub(self) mod trivial;

pub(in crate::circuits) trait PostImageTableConfigTrait<F: FieldExt> {
    fn configure(_meta: &mut ConstraintSystem<F>) -> Self;
}

pub(in crate::circuits) trait PostImageTableChipTrait<
    F: FieldExt,
    Config: PostImageTableConfigTrait<F>,
>
{
    fn new(config: Config) -> Self;
    fn assign(
        self,
        layouter: &mut impl Layouter<F>,
        image_table: ImageTableLayouter<F>,
        permutation_cells: ImageTableLayouter<Cell>,
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
