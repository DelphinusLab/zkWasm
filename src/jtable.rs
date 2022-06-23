use crate::itable::Inst;
use crate::utils::bn_to_field;
use crate::utils::Context;
use halo2_proofs::arithmetic::FieldExt;
use halo2_proofs::plonk::Advice;
use halo2_proofs::plonk::Column;
use halo2_proofs::plonk::Error;
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
}

pub struct JumpTableConfig {
    cols: [Column<Advice>; 3],
}

pub struct EventTableChip<F: FieldExt> {
    config: JumpTableConfig,
    _phantom: PhantomData<F>,
}

impl<F: FieldExt> EventTableChip<F> {
    pub fn new(config: JumpTableConfig) -> Self {
        EventTableChip {
            config,
            _phantom: PhantomData,
        }
    }

    pub fn add_jump(&self, ctx: &mut Context<'_, F>, jump: Box<Jump>) -> Result<(), Error> {
        ctx.region.assign_advice_from_constant(
            || "jump eid",
            self.config.cols[0],
            ctx.offset,
            F::from(jump.eid),
        )?;
        ctx.region.assign_advice_from_constant(
            || "jump last_jump_eid",
            self.config.cols[1],
            ctx.offset,
            F::from(jump.last_jump_eid),
        )?;
        ctx.region.assign_advice_from_constant(
            || "jump addr",
            self.config.cols[2],
            ctx.offset,
            bn_to_field(&jump.inst.encode_addr()),
        )?;
        Ok(())
    }
}
