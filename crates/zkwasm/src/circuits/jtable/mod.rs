use self::configure::JTableConstraint;
use halo2_proofs::arithmetic::FieldExt;
use halo2_proofs::plonk::Advice;
use halo2_proofs::plonk::Column;
use halo2_proofs::plonk::ConstraintSystem;
use halo2_proofs::plonk::Fixed;
use num_bigint::BigUint;
use num_bigint::ToBigUint;
use specs::jtable::STATIC_FRAME_ENTRY_NUMBER;
use std::marker::PhantomData;

mod assign;
mod configure;
pub(crate) mod expression;

// enable and data should be encoded in image table
pub(crate) const STATIC_FRAME_ENTRY_IMAGE_TABLE_ENTRY: usize = STATIC_FRAME_ENTRY_NUMBER * 2;

// high 128 bit counts 'return' instructions, low 128 bit counts 'call' instructions.
pub(crate) const JOPS_SEPARATE: usize = 128;
pub fn encode_jops(return_instructions: u32, call_instructions: u32) -> BigUint {
    return_instructions.to_biguint().unwrap() << JOPS_SEPARATE
        | call_instructions.to_biguint().unwrap()
}

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
