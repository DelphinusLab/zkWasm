use crate::itable::Inst;
use crate::utils::bn_to_field;
use crate::utils::Context;
use halo2_proofs::arithmetic::FieldExt;
use halo2_proofs::plonk::Advice;
use halo2_proofs::plonk::Column;
use halo2_proofs::plonk::Error;
use num_bigint::BigUint;
use std::marker::PhantomData;

pub struct Jump {
    eid: u64,
    last_jump_eid: u64,
    inst: Box<Inst>,
}

impl Jump {
    pub fn new(eid: u64, last_jump_eid: u64, inst: Box<Inst>) -> Jump {
        Jump {
            eid,
            last_jump_eid,
            inst,
        }
    }

    pub fn encode(&self) -> BigUint {
        todo!()
    }
}

#[derive(Clone)]
pub struct JumpTableConfig<F: FieldExt> {
    col: Column<Advice>,
    _mark: PhantomData<F>,
}

impl<F: FieldExt> JumpTableConfig<F> {
    pub fn new(
        cols: &mut impl Iterator<Item = halo2_proofs::plonk::Column<halo2_proofs::plonk::Advice>>,
    ) -> Self {
        Self {
            col: cols.next().unwrap(),
            _mark: PhantomData,
        }
    }
}

pub struct EventTableChip<F: FieldExt> {
    config: JumpTableConfig<F>,
}

impl<F: FieldExt> EventTableChip<F> {
    pub fn new(config: JumpTableConfig<F>) -> Self {
        EventTableChip { config }
    }

    pub fn add_jump(&self, ctx: &mut Context<'_, F>, jump: Box<Jump>) -> Result<(), Error> {
        ctx.region.assign_advice_from_constant(
            || "jump table entry",
            self.config.col,
            ctx.offset,
            bn_to_field(&jump.encode()),
        )?;
        Ok(())
    }
}
