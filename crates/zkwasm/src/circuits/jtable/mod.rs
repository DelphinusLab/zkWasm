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

#[repr(i32)]
pub(self) enum FrameTableValueOffset {
    Enable = 0,
    Returned = 1,
    Encode = 2,
    CallOps = 3,
    ReturnOps = 4,
    Max = 5,
}

#[derive(Clone)]
pub struct JumpTableConfig<F: FieldExt> {
    sel: Column<Fixed>,
    inherited: Column<Fixed>,
    value: Column<Advice>,
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
