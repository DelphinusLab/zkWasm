use self::configure::JTableConstraint;
use super::config::max_jtable_rows;
use halo2_proofs::{
    arithmetic::FieldExt,
    plonk::{Advice, Column, ConstraintSystem, Fixed},
};
use std::marker::PhantomData;

mod assign;
mod configure;
pub(crate) mod expression;

pub enum JtableOffset {
    JtableOffsetEnable = 0,
    JtableOffsetRest = 1,
    JtableOffsetEntry = 2,
    JtableOffsetMax = 3,
}

fn jtable_rows() -> usize {
    max_jtable_rows() as usize / JtableOffset::JtableOffsetMax as usize
        * JtableOffset::JtableOffsetMax as usize
}

#[derive(Clone)]
pub struct JumpTableConfig<F: FieldExt> {
    sel: Column<Fixed>,
    static_bit: Column<Fixed>,
    data: Column<Advice>,
    _m: PhantomData<F>,
}

impl<F: FieldExt> JumpTableConfig<F> {
    pub fn configure(
        meta: &mut ConstraintSystem<F>,
        cols: &mut impl Iterator<Item = Column<Advice>>,
    ) -> Self {
        let jtable = Self::new(meta, cols);
        jtable.configure(meta);
        jtable
    }
}

pub struct JumpTableChip<F: FieldExt> {
    config: JumpTableConfig<F>,
}

impl<F: FieldExt> JumpTableChip<F> {
    pub fn new(config: JumpTableConfig<F>) -> Self {
        JumpTableChip { config }
    }
}
