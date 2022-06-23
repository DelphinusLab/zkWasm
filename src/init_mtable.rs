use halo2_proofs::{
    arithmetic::FieldExt,
    plonk::{Column, Error, Fixed},
};
use std::marker::PhantomData;

use crate::utils::Context;

pub struct MInit {
    mmid: u64,
    offset: u64,
    value: u64,
}

pub const MINIT_TABLE_COLUMNS: usize = 3usize;

pub struct MInitTableConfig {
    cols: [Column<Fixed>; MINIT_TABLE_COLUMNS],
}

impl MInitTableConfig {
    pub fn new(cols: [Column<Fixed>; MINIT_TABLE_COLUMNS]) -> Self {
        Self { cols }
    }
}

pub struct MInitTableChip<F: FieldExt> {
    config: MInitTableConfig,
    _phantom: PhantomData<F>,
}

impl<F: FieldExt> MInitTableChip<F> {
    pub fn add_memory_init(self, ctx: &mut Context<'_, F>, minit: MInit) -> Result<(), Error> {
        ctx.region.assign_fixed(
            || "minit mmid",
            self.config.cols[0],
            ctx.offset,
            || Ok(F::from(minit.mmid)),
        )?;
        ctx.region.assign_fixed(
            || "minit offset",
            self.config.cols[1],
            ctx.offset,
            || Ok(F::from(minit.offset)),
        )?;
        ctx.region.assign_fixed(
            || "minit value",
            self.config.cols[2],
            ctx.offset,
            || Ok(F::from(minit.value)),
        )?;
        Ok(())
    }
}
