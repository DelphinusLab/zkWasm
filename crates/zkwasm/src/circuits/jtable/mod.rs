use self::configure::JTableConstraint;
use halo2_proofs::arithmetic::FieldExt;
use halo2_proofs::circuit::AssignedCell;
use halo2_proofs::plonk::Advice;
use halo2_proofs::plonk::Column;
use halo2_proofs::plonk::ConstraintSystem;
use halo2_proofs::plonk::Fixed;
use std::marker::PhantomData;

mod assign;
mod configure;
pub(crate) mod expression;

#[derive(Clone)]
pub struct JumpTableConfig<F: FieldExt> {
    sel: Column<Fixed>,

    inherited: Column<Fixed>,

    enable: Column<Advice>,
    returned: Column<Advice>,
    encode: Column<Advice>,

    call_ops: Column<Advice>,
    return_ops: Column<Advice>,
    _m: PhantomData<F>,
}

impl<F: FieldExt> JumpTableConfig<F> {
    pub fn configure(meta: &mut ConstraintSystem<F>, is_last_slice: bool) -> Self {
        let jtable = Self::new(meta);
        jtable.configure(meta, is_last_slice);
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

#[derive(Debug)]
pub(crate) struct FrameEtablePermutationCells<F: FieldExt> {
    pub(crate) rest_call_ops: AssignedCell<F, F>,
    pub(crate) rest_return_ops: AssignedCell<F, F>,
}
