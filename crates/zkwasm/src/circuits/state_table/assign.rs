use halo2_proofs::arithmetic::FieldExt;
use halo2_proofs::circuit::Cell;
use halo2_proofs::circuit::Layouter;
use halo2_proofs::plonk::Error;
use specs::InitializationState;

use super::StateTableChip;
#[cfg(feature = "continuation")]
use crate::circuits::utils::Context;

impl<F: FieldExt> StateTableChip<F> {
    #[cfg(not(feature = "continuation"))]
    pub fn assign(
        self,
        _layouter: &mut impl Layouter<F>,
        _state_initialization: &InitializationState<u32>,
        _permutation_cells: &InitializationState<Cell>,
    ) -> Result<(), Error> {
        Ok(())
    }

    #[cfg(feature = "continuation")]
    pub fn assign(
        self,
        layouter: &mut impl Layouter<F>,
        state_initialization: &InitializationState<u32>,
        permutation_cells: &InitializationState<Cell>,
    ) -> Result<(), Error> {
        layouter.assign_region(
            || "image table",
            |region| {
                let mut ctx = Context::new(region);

                macro_rules! assign_and_constrain_state {
                    ($f: ident) => {
                        let cell = ctx
                            .region
                            .assign_advice(
                                || "image table: state initialization",
                                self.config.col,
                                ctx.offset,
                                || Ok(F::from(state_initialization.$f as u64)),
                            )?
                            .cell();
                        ctx.region.constrain_equal(cell, permutation_cells.$f)?;
                        ctx.next();
                    };
                }

                assign_and_constrain_state!(eid);
                assign_and_constrain_state!(fid);
                assign_and_constrain_state!(iid);
                assign_and_constrain_state!(frame_id);
                assign_and_constrain_state!(sp);

                assign_and_constrain_state!(initial_memory_pages);

                // TODO: open rest_mops
                // pub rest_mops: Option<T>,
                assign_and_constrain_state!(rest_jops);

                Ok(())
            },
        )
    }
}
