use super::Sha256HelperOp;
use crate::{
    circuits::shared_column_pool::{DynTableLookupColumn, SharedColumnPool},
    constant_from, fixed_curr,
    foreign::ForeignTableConfig,
    traits::circuits::bit_range_table::{BitColumn, BitRangeTable},
};
use halo2_proofs::{
    arithmetic::FieldExt,
    plonk::{Advice, Column, ConstraintSystem, Expression, Fixed, TableColumn, VirtualCells},
};
use std::marker::PhantomData;

pub mod assign;
pub mod config;
pub mod expr;
pub mod ops;

const OP_ARGS_NUM: usize = 5;
const K: usize = 15;
pub const ENABLE_LINES: usize = 1 << (K - 1);
const BLOCK_LINES: usize = 10;

pub struct Sha2HelperEncode();

impl Sha2HelperEncode {
    pub(super) fn encode_opcode_expr<F: FieldExt>(
        op: Expression<F>,
        args: Vec<Expression<F>>,
        ret: Expression<F>,
    ) -> Expression<F> {
        assert!(args.len() < OP_ARGS_NUM);
        let mut acc = op * constant_from!(1 << (OP_ARGS_NUM * 4));
        for (i, v) in args.into_iter().enumerate() {
            acc = acc + v * constant_from!(1 << (i * 4 + 4));
        }
        acc = acc + ret;
        acc
    }

    pub(super) fn encode_opcode_f<F: FieldExt>(op: Sha256HelperOp, args: &Vec<u32>, ret: u32) -> F {
        assert!(args.len() < OP_ARGS_NUM);
        let mut acc = F::from(op as u64) * F::from(1u64 << (OP_ARGS_NUM * 4));
        for (i, v) in args.into_iter().enumerate() {
            acc = acc + F::from(*v as u64) * F::from(1u64 << (i * 4 + 4));
        }
        acc = acc + F::from(ret as u64);
        acc
    }

    pub(super) fn encode_table_f<F: FieldExt>(op: Sha256HelperOp, args: [u32; 3], ret: u32) -> F {
        let mut acc = F::from(op as u64) * F::from(1u64 << (OP_ARGS_NUM * 4));
        for (i, v) in args.into_iter().enumerate() {
            acc = acc + F::from(v as u64) * F::from(1u64 << (i * 4 + 4));
        }
        acc = acc + F::from(ret as u64);
        acc
    }

    pub(super) fn encode_table_expr<F: FieldExt>(
        op: Expression<F>,
        args: [Expression<F>; 3],
        ret: Expression<F>,
    ) -> Expression<F> {
        let mut acc = op * constant_from!(1u64 << (OP_ARGS_NUM * 4));
        for (i, v) in args.into_iter().enumerate() {
            acc = acc + v * constant_from!(1u64 << (i * 4 + 4));
        }
        acc = acc + ret;
        acc
    }
}

#[derive(Clone)]
pub struct Sha256HelperTableConfig<F: FieldExt> {
    sel: Column<Fixed>,
    block_first_line_sel: Column<Fixed>,

    op_bit: BitColumn,
    op: Column<Advice>,
    args: [Column<Advice>; OP_ARGS_NUM],
    aux: DynTableLookupColumn<F>, // limited to u8 except for block first line

    op_valid_set: TableColumn,
    mark: PhantomData<F>,
}

impl<F: FieldExt> Sha256HelperTableConfig<F> {
    fn new(
        meta: &mut ConstraintSystem<F>,
        shared_column_pool: &SharedColumnPool<F>,
        rtable: &impl BitRangeTable<F>,
    ) -> Self {
        let sel = meta.fixed_column();
        let block_first_line_sel = meta.fixed_column();
        let op = shared_column_pool.acquire_u8_col(0);
        let op_bit = rtable.bit_column(meta, "sha256 helper op_bit", |meta| fixed_curr!(meta, sel));
        let args = [0, 1, 2, 3, 4].map(|i| shared_column_pool.acquire_u4_col(i));
        let aux = shared_column_pool.acquire_dyn_col(0);
        let op_valid_set = meta.lookup_table_column();

        Self {
            sel,
            block_first_line_sel,
            op_bit,
            op,
            args,
            aux,
            op_valid_set,
            mark: PhantomData,
        }
    }

    pub fn configure(
        meta: &mut ConstraintSystem<F>,
        shared_column_pool: &SharedColumnPool<F>,
        rtable: &impl BitRangeTable<F>,
    ) -> Self {
        let config = Self::new(meta, shared_column_pool, rtable);
        config._configure(meta);
        config
    }
}

impl<F: FieldExt> ForeignTableConfig<F> for Sha256HelperTableConfig<F> {
    fn configure_in_table(
        &self,
        meta: &mut ConstraintSystem<F>,
        key: &'static str,
        expr: &dyn Fn(&mut VirtualCells<'_, F>) -> Expression<F>,
    ) {
        meta.lookup_any(key, |meta| {
            vec![(
                expr(meta),
                fixed_curr!(meta, self.block_first_line_sel)
                    * self.is_block_enabled_expr(meta)
                    * self.opcode_expr(meta),
            )]
        });
    }
}
