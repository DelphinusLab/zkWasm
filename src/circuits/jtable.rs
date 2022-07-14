use super::rtable::RangeTableConfig;
use super::utils::bn_to_field;
use super::utils::Context;
use super::Encode;
use crate::constant;
use crate::constant_from;
use crate::curr;
use crate::next;
use halo2_proofs::arithmetic::FieldExt;
use halo2_proofs::circuit::Cell;
use halo2_proofs::plonk::Advice;
use halo2_proofs::plonk::Column;
use halo2_proofs::plonk::ConstraintSystem;
use halo2_proofs::plonk::Error;
use halo2_proofs::plonk::Expression;
use halo2_proofs::plonk::VirtualCells;
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

    pub fn configure_in_table(
        &self,
        meta: &mut ConstraintSystem<F>,
        eid: impl FnOnce(&mut VirtualCells<'_, F>) -> Expression<F>,
        last_jump_eid: impl FnOnce(&mut VirtualCells<'_, F>) -> Expression<F>,
        moid: impl FnOnce(&mut VirtualCells<'_, F>) -> Expression<F>,
        fid: impl FnOnce(&mut VirtualCells<'_, F>) -> Expression<F>,
        iid: impl FnOnce(&mut VirtualCells<'_, F>) -> Expression<F>,
    ) {
        let one = BigUint::from(1u64);
        meta.lookup_any("jtable lookup", |meta| {
            vec![(
                eid(meta) * constant!(bn_to_field(&(&one << EID_SHIFT)))
                    + last_jump_eid(meta) * constant!(bn_to_field(&(&one << LAST_JUMP_EID_SHIFT)))
                    + moid(meta) * constant!(bn_to_field(&(&one << MOID_SHIFT)))
                    + fid(meta) * constant!(bn_to_field(&(&one << FID_SHIFT)))
                    + iid(meta),
                curr!(meta, self.entry),
            )]
        });
    }
}

const EID_SHIFT: usize = 96;
const LAST_JUMP_EID_SHIFT: usize = 80;
const MOID_SHIFT: usize = 32;
const FID_SHIFT: usize = 32;

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
