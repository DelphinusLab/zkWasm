use halo2_proofs::arithmetic::FieldExt;
use halo2_proofs::plonk::Column;
use halo2_proofs::plonk::Error;
use halo2_proofs::plonk::Fixed;
use num_bigint::BigUint;
use num_traits::identities::Zero;
use std::marker::PhantomData;
use wasmi::tracer::itable::IEntry;

use crate::utils::bn_to_field;
use crate::utils::Context;

pub struct Inst {
    moid: u16,
    fid: u16,
    bid: u16,
    iid: u16,
    opcode: u64,
}

impl From<IEntry> for Inst {
    fn from(i_entry: IEntry) -> Self {
        Inst {
            moid: i_entry.module_instance_index,
            fid: i_entry.func_index,
            bid: 0,
            iid: i_entry.pc,
            opcode: i_entry.opcode,
        }
    }
}

impl Inst {
    pub fn new(moid: u16, fid: u16, bid: u16, iid: u16, opcode: u64) -> Self {
        Inst {
            moid,
            fid,
            bid,
            iid,
            opcode,
        }
    }

    pub fn encode(&self) -> BigUint {
        let mut bn = BigUint::zero();
        bn <<= 16u8;
        bn += self.moid;
        bn <<= 16u8;
        bn += self.fid;
        bn <<= 16u8;
        bn += self.bid;
        bn <<= 16u8;
        bn += self.iid;
        bn <<= 64u8;
        bn += self.opcode;
        bn
    }
}

pub struct InstTableConfig {
    col: Column<Fixed>,
}

pub struct InstTableChip<F: FieldExt> {
    config: InstTableConfig,
    _phantom: PhantomData<F>,
}

impl<F: FieldExt> InstTableChip<F> {
    pub fn add_inst(&self, ctx: &mut Context<'_, F>, inst: Inst) -> Result<(), Error> {
        ctx.region.assign_fixed(
            || "inst table",
            self.config.col,
            ctx.offset,
            || Ok(bn_to_field(&inst.encode())),
        )?;
        ctx.offset += 1;
        Ok(())
    }
}
