use super::rtable::RangeTableConfig;
use super::utils::bn_to_field;
use super::utils::Context;
use super::Encode;
use crate::constant_from;
use crate::curr;
use crate::next;
use halo2_proofs::arithmetic::FieldExt;
use halo2_proofs::circuit::Cell;
use halo2_proofs::plonk::Advice;
use halo2_proofs::plonk::Column;
use halo2_proofs::plonk::ConstraintSystem;
use halo2_proofs::plonk::Error;
use num_bigint::BigUint;
use specs::jtable::JumpTableEntry;
use std::marker::PhantomData;
use std::vec;

impl Encode for JumpTableEntry {
    fn encode(&self) -> BigUint {
        todo!()
    }
}

#[derive(Clone)]
pub struct JumpTableConfig<F: FieldExt> {
    rest: Column<Advice>,
    entry: Column<Advice>,
    _mark: PhantomData<F>,
}

impl<F: FieldExt> JumpTableConfig<F> {
    pub fn configure(
        meta: &mut ConstraintSystem<F>,
        cols: &mut impl Iterator<Item = Column<Advice>>,
        rtable: &RangeTableConfig<F>,
    ) -> Self {
        let rest = cols.next().unwrap();
        let entry = cols.next().unwrap();

        meta.create_gate("jtable rest decrease", |meta| {
            vec![(curr!(meta, rest) - next!(meta, rest) - constant_from!(2)) * curr!(meta, entry)]
        });

        meta.enable_equality(rest);

        rtable.configure_in_common_range(meta, "jtable rest in common range", |meta| {
            curr!(meta, rest)
        });

        Self {
            rest,
            entry,
            _mark: PhantomData,
        }
    }
}

pub struct JumpTableChip<F: FieldExt> {
    config: JumpTableConfig<F>,
}

impl<F: FieldExt> JumpTableChip<F> {
    pub fn new(config: JumpTableConfig<F>) -> Self {
        JumpTableChip { config }
    }

    pub fn assign(
        &self,
        ctx: &mut Context<'_, F>,
        entries: &Vec<JumpTableEntry>,
        etable_rest_jops_cell: Cell,
    ) -> Result<(), Error> {
        let mut rest = entries.len() as u64 * 2;
        for (i, entry) in entries.iter().enumerate() {
            let cell = ctx.region.assign_advice(
                || "jtable rest",
                self.config.rest,
                ctx.offset,
                || Ok(rest.into()),
            )?;

            if i == 0 {
                ctx.region
                    .constrain_equal(cell.cell(), etable_rest_jops_cell)?;
            }

            ctx.region.assign_advice(
                || "jtable entry",
                self.config.entry,
                ctx.offset,
                || Ok(bn_to_field(&entry.inst.encode_instruction_address())),
            )?;

            rest -= 2;
            ctx.next()
        }
        Ok(())
    }
}
