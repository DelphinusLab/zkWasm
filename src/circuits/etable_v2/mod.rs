use std::{collections::HashSet, marker::PhantomData};

use halo2_proofs::{
    arithmetic::FieldExt,
    plonk::{Advice, Column, ConstraintSystem, Fixed},
};
use specs::itable::OpcodeClassPlain;

use self::allocator::{CellAllocator, ETableCellType};

use super::{rtable::RangeTableConfig, CircuitConfigure};

mod allocator;

pub(crate) const ESTEP_SIZE: i32 = 4;

pub struct ETableConfig<F: FieldExt> {
    pub sel: Column<Fixed>,
    pub step_sel: Column<Fixed>,
    _mark: PhantomData<F>,
}

impl<F: FieldExt> ETableConfig<F> {
    pub(crate) fn configure(
        meta: &mut ConstraintSystem<F>,
        cols: &mut (impl Iterator<Item = Column<Advice>> + Clone),
        _circuit_configure: &CircuitConfigure,
        rtable: &RangeTableConfig<F>,
        _opcode_set: &HashSet<OpcodeClassPlain>,
    ) -> ETableConfig<F> {
        let sel = meta.fixed_column();
        let step_sel = meta.fixed_column();

        let mut allocator = CellAllocator::new(meta, rtable, cols);
        allocator.enable_equality(meta, &ETableCellType::CommonRange);

        Self {
            sel,
            step_sel,
            _mark: PhantomData,
        }
    }
}
