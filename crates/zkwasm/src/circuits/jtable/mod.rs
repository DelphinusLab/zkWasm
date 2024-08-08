use self::configure::JTableConstraint;
use halo2_proofs::arithmetic::FieldExt;
use halo2_proofs::plonk::Advice;
use halo2_proofs::plonk::Column;
use halo2_proofs::plonk::ConstraintSystem;
use halo2_proofs::plonk::Fixed;
use std::marker::PhantomData;

mod assign;
mod configure;
pub(crate) mod expression;

// high 128 bit counts 'return' instructions, low 128 bit counts 'call' instructions.
pub(crate) const JOPS_SEPARATE: usize = 128;

pub enum JtableOffset {
    JtableOffsetEnable = 0,
    JtableOffsetRest = 1,
    JtableOffsetEntry = 2,
    JtableOffsetMax = 3,
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
    max_available_rows: usize,
}

impl<F: FieldExt> JumpTableChip<F> {
    pub fn new(config: JumpTableConfig<F>, max_available_rows: usize) -> Self {
        JumpTableChip {
            config,
            max_available_rows,
        }
    }
}
