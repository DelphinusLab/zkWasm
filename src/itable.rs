use crate::constant;
use crate::utils::bn_to_field;
use halo2_proofs::arithmetic::FieldExt;
use halo2_proofs::circuit::Layouter;
use halo2_proofs::plonk::Advice;
use halo2_proofs::plonk::Column;
use halo2_proofs::plonk::ConstraintSystem;
use halo2_proofs::plonk::Error;
use halo2_proofs::plonk::Expression;
use halo2_proofs::plonk::TableColumn;
use halo2_proofs::plonk::VirtualCells;
use num_bigint::BigUint;
use num_traits::identities::Zero;
use num_traits::One;
use std::marker::PhantomData;
use wasmi::tracer::itable::IEntry;

pub struct Inst {
    moid: u16,
    pub(crate) mmid: u16,
    fid: u16,
    bid: u16,
    iid: u16,
    opcode: u64,
    aux: u64,
}

impl From<&IEntry> for Inst {
    fn from(i_entry: &IEntry) -> Self {
        Inst {
            moid: i_entry.module_instance_index,
            //TODO: cover import
            mmid: i_entry.module_instance_index,
            fid: i_entry.func_index,
            bid: 0,
            iid: i_entry.pc,
            opcode: i_entry.opcode,
            aux: 0,
        }
    }
}

impl Inst {
    pub fn new(moid: u16, mmid: u16, fid: u16, bid: u16, iid: u16, opcode: u64, aux: u64) -> Self {
        Inst {
            moid,
            mmid,
            fid,
            bid,
            iid,
            opcode,
            aux,
        }
    }

    pub fn encode(&self) -> BigUint {
        let mut bn = self.encode_addr();
        bn <<= 64u8;
        bn += self.opcode;
        bn <<= 64u8;
        bn += self.aux;
        bn
    }

    pub fn encode_addr(&self) -> BigUint {
        let mut bn = BigUint::zero();
        bn += self.moid;
        bn <<= 16u8;
        bn += self.mmid;
        bn <<= 16u8;
        bn += self.fid;
        bn <<= 16u8;
        bn += self.bid;
        bn <<= 16u8;
        bn += self.iid;
        bn
    }
}

pub fn encode_inst_expr<F: FieldExt>(
    moid: Expression<F>,
    mmid: Expression<F>,
    fid: Expression<F>,
    bid: Expression<F>,
    iid: Expression<F>,
    opcode: Expression<F>,
) -> Expression<F> {
    let mut bn = BigUint::one();
    let mut acc = opcode;
    bn <<= 64u8;
    acc = acc + iid * constant!(bn_to_field(&bn));
    bn <<= 16u8;
    acc = acc + bid * constant!(bn_to_field(&bn));
    bn <<= 16u8;
    acc = acc + fid * constant!(bn_to_field(&bn));
    bn <<= 16u8;
    acc = acc + mmid * constant!(bn_to_field(&bn));
    bn <<= 16u8;
    acc = acc + moid * constant!(bn_to_field(&bn));

    acc
}

#[derive(Clone)]
pub struct InstTableConfig<F: FieldExt> {
    col: TableColumn,
    _mark: PhantomData<F>,
}

impl<F: FieldExt> InstTableConfig<F> {
    pub fn new(meta: &mut ConstraintSystem<F>) -> Self {
        InstTableConfig {
            col: meta.lookup_table_column(),
            _mark: PhantomData,
        }
    }

    pub fn configure_in_table(
        &self,
        meta: &mut ConstraintSystem<F>,
        key: &'static str,
        expr: impl FnOnce(&mut VirtualCells<'_, F>) -> Expression<F>,
    ) {
        meta.lookup(key, |meta| vec![(expr(meta), self.col)]);
    }
}

#[derive(Clone)]
pub struct InstTableChip<F: FieldExt> {
    config: InstTableConfig<F>,
}

impl<F: FieldExt> InstTableChip<F> {
    pub fn new(meta: &mut ConstraintSystem<F>) -> Self {
        InstTableChip {
            config: InstTableConfig {
                col: meta.lookup_table_column(),
                _mark: PhantomData,
            },
        }
    }

    pub fn add_inst_init(
        self,
        layouter: &mut impl Layouter<F>,
        inst_init: Vec<Inst>,
    ) -> Result<(), Error> {
        layouter.assign_table(
            || "inst_init",
            |mut table| {
                for (i, v) in inst_init.iter().enumerate() {
                    table.assign_cell(
                        || "inst_init talbe",
                        self.config.col,
                        i,
                        || Ok(bn_to_field::<F>(&v.encode())),
                    )?;
                }
                Ok(())
            },
        )?;
        Ok(())
    }
}
