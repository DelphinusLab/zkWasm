use halo2_proofs::arithmetic::FieldExt;
use halo2_proofs::plonk::Advice;
use halo2_proofs::plonk::Column;
use halo2_proofs::plonk::ConstraintSystem;
use halo2_proofs::plonk::Expression;
use halo2_proofs::plonk::VirtualCells;

use crate::constant_from;
use crate::curr;
use crate::fixed_curr;

#[derive(Clone)]
pub struct U4Column(pub Column<Advice>);

#[derive(Clone)]
pub struct U8Column(pub Column<Advice>);

#[derive(Clone)]
pub struct BitColumn(pub Column<Advice>);

#[derive(Clone)]
pub struct U4PartialColumn(pub Column<Advice>);

#[derive(Clone)]
pub struct U8PartialColumn(pub Column<Advice>);

#[derive(Clone)]
pub struct BitPartialColumn(pub Column<Advice>);

pub trait BitRangeTable<F: FieldExt> {
    fn configure_in_u4_range(
        &self,
        meta: &mut ConstraintSystem<F>,
        key: &'static str,
        expr: impl FnOnce(&mut VirtualCells<'_, F>) -> Expression<F>,
    );
    fn configure_in_u8_range(
        &self,
        meta: &mut ConstraintSystem<F>,
        key: &'static str,
        expr: impl FnOnce(&mut VirtualCells<'_, F>) -> Expression<F>,
    );
    fn u4_partial_column(
        &self,
        meta: &mut ConstraintSystem<F>,
        key: &'static str,
        filter: impl FnOnce(&mut VirtualCells<'_, F>) -> Expression<F>,
    ) -> U4PartialColumn {
        let col = meta.advice_column();
        self.configure_in_u4_range(meta, key, |meta| curr!(meta, col) * filter(meta));
        U4PartialColumn(col)
    }
    fn u8_partial_column(
        &self,
        meta: &mut ConstraintSystem<F>,
        key: &'static str,
        filter: impl FnOnce(&mut VirtualCells<'_, F>) -> Expression<F>,
    ) -> U8PartialColumn {
        let col = meta.advice_column();
        self.configure_in_u8_range(meta, key, |meta| curr!(meta, col) * filter(meta));
        U8PartialColumn(col)
    }
    fn bit_partial_column(
        &self,
        meta: &mut ConstraintSystem<F>,
        key: &'static str,
        filter: impl FnOnce(&mut VirtualCells<'_, F>) -> Expression<F>,
    ) -> BitPartialColumn {
        let col = meta.advice_column();
        meta.create_gate(key, |meta| {
            vec![curr!(meta, col) * (constant_from!(1) - curr!(meta, col)) * filter(meta)]
        });
        BitPartialColumn(col)
    }
    fn u4_column(
        &self,
        meta: &mut ConstraintSystem<F>,
        key: &'static str,
        sel: impl FnOnce(&mut VirtualCells<'_, F>) -> Expression<F>,
    ) -> U4Column {
        let col = meta.advice_column();
        self.configure_in_u4_range(meta, key, |meta| sel(meta) * curr!(meta, col));
        U4Column(col)
    }
    fn u8_column(
        &self,
        meta: &mut ConstraintSystem<F>,
        key: &'static str,
        sel: impl FnOnce(&mut VirtualCells<'_, F>) -> Expression<F>,
    ) -> U8Column {
        let col = meta.advice_column();
        self.configure_in_u8_range(meta, key, |meta| sel(meta) * curr!(meta, col));
        U8Column(col)
    }
    fn bit_column(
        &self,
        meta: &mut ConstraintSystem<F>,
        key: &'static str,
        sel: impl FnOnce(&mut VirtualCells<'_, F>) -> Expression<F>,
    ) -> BitColumn {
        let col = meta.advice_column();
        meta.create_gate(key, |meta| {
            vec![sel(meta) * curr!(meta, col) * (constant_from!(1) - curr!(meta, col))]
        });
        BitColumn(col)
    }
}
