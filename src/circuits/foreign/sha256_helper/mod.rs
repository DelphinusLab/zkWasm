use crate::{circuits::rtable::RangeTableConfig, constant_from, curr, fixed_curr, nextn};
use halo2_proofs::{
    arithmetic::FieldExt,
    plonk::{Advice, Column, ConstraintSystem, Expression, Fixed, TableColumn, VirtualCells},
};
use std::marker::PhantomData;

const OP_ARGS_NUM: usize = 5;
const BLOCK_LINES: usize = 8;

struct Sha2HelperConfig<F: FieldExt> {
    sel: Column<Fixed>,
    block_first_line_sel: Column<Fixed>,
    op: Column<Advice>,
    args: [Column<Advice>; OP_ARGS_NUM],
    aux: Column<Advice>, // limited to u8 except for block first line
    op_bit: Column<Advice>,

    op_valid_set: TableColumn,
    mark: PhantomData<F>,
}

enum Sha256HelperOp {
    Ch = 1,
    Ma,
    LSigma0,
    LSigma1,
    SSigma0,
    SSigms1,
}

fn encode_op_expr<F: FieldExt>(
    op: Expression<F>,
    args: [&Expression<F>; OP_ARGS_NUM],
) -> Expression<F> {
    let mut acc = op * constant_from!(1 << (OP_ARGS_NUM * 4));
    for i in 0..OP_ARGS_NUM {
        acc = acc + args[i].clone() * constant_from!(1 << (i * 4));
    }
    acc
}

impl<F: FieldExt> Sha2HelperConfig<F> {
    fn new(meta: &mut ConstraintSystem<F>) -> Self {
        let sel = meta.fixed_column();
        let block_first_line_sel = meta.fixed_column();
        let op = meta.advice_column();
        let op_bit = meta.advice_column();
        let args = [0; OP_ARGS_NUM].map(|_| meta.advice_column());
        let aux = meta.advice_column();
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

    pub fn configure(meta: &mut ConstraintSystem<F>, rtable: &RangeTableConfig<F>) -> Self {
        let config = Self::new(meta);

        rtable.configure_in_u8_range(
            meta,
            "sha2 aux in u8 except for the block first line",
            |meta| {
                fixed_curr!(meta, config.sel)
                    * (fixed_curr!(meta, config.block_first_line_sel) - constant_from!(1))
                    * curr!(meta, config.aux)
            },
        );

        rtable.configure_in_u8_range(meta, "sha2 op in u8", |meta| curr!(meta, config.op));
        for i in 0..OP_ARGS_NUM {
            rtable
                .configure_in_u4_range(meta, "sha2 args in u4", |meta| curr!(meta, config.args[i]));
        }

        meta.create_gate("sha2 helper op_bits", |meta| {
            let mut sum = curr!(meta, config.op_bit);
            for i in 1..BLOCK_LINES as i32 {
                sum = sum + nextn!(meta, config.op_bit, i);
            }

            vec![
                fixed_curr!(meta, config.sel)
                    * curr!(meta, config.op_bit)
                    * (constant_from!(1) - curr!(meta, config.op_bit)),
                fixed_curr!(meta, config.sel) * (constant_from!(1) - sum),
            ]
        });

        meta.create_gate("sha2 op eq", |meta| {
            let mut acc = curr!(meta, config.op) * constant_from!(15);
            for i in 1..BLOCK_LINES as i32 {
                acc = acc - nextn!(meta, config.op, i);
            }
            vec![fixed_curr!(meta, config.sel) * acc]
        });

        meta.lookup("sha2 op lookup", |meta| {
            let mut pos_acc = 1 << (4 * OP_ARGS_NUM);
            let mut acc = curr!(meta, config.op) * constant_from!(pos_acc);

            for i in 0..OP_ARGS_NUM {
                pos_acc >>= 4;
                acc = acc + curr!(meta, config.args[i]) * constant_from!(pos_acc);
            }

            vec![(fixed_curr!(meta, config.sel) * acc, config.op_valid_set)]
        });

        // TODO: add config for each op.

        config
    }

    fn arg_to_u32(
        &self,
        meta: &mut VirtualCells<'_, F>,
        index: usize,
        start: i32,
    ) -> Expression<F> {
        assert!(start < BLOCK_LINES as i32);
        let mut shift_acc = 1;
        let mut acc = nextn!(meta, self.args[index], start);

        for i in start + 1..BLOCK_LINES as i32 {
            shift_acc += 4;
            acc = acc + nextn!(meta, self.args[index], i) * constant_from!(1u64 << shift_acc);
        }

        acc
    }

    fn op_code(&self, meta: &mut VirtualCells<'_, F>) -> Expression<F> {
        fixed_curr!(meta, self.block_first_line_sel) * curr!(meta, self.aux)
    }

    fn configure_SSigma0(&self, meta: &mut ConstraintSystem<F>, index: i32) {
        let enable = |meta: &mut VirtualCells<F>| {
            fixed_curr!(meta, self.block_first_line_sel) * nextn!(meta, self.op_bit, index)
        };

        // s0 = (x >> 7) xor (x >> 18) xor (x >> 3)
        meta.create_gate("sha256 s0", |meta| {
            let x = self.arg_to_u32(meta, 0, 0);
            let x16 = self.arg_to_u32(meta, 0, 4);

            let x7 = self.arg_to_u32(meta, 1, 0);
            let x18 = self.arg_to_u32(meta, 2, 0);
            let x3 = self.arg_to_u32(meta, 3, 0);

            let x7_helper = nextn!(meta, self.aux, 1);
            let x7_helper_diff = nextn!(meta, self.aux, 2);
            let x18_helper = nextn!(meta, self.aux, 3);
            let x18_helper_diff = nextn!(meta, self.aux, 4);
            let x3_helper = nextn!(meta, self.aux, 5);
            let x3_helper_diff = nextn!(meta, self.aux, 6);

            vec![
                enable(meta) * (x.clone() - x7 * constant_from!(1 << 7) - x7_helper.clone()),
                enable(meta) * (x7_helper.clone() + x7_helper_diff - constant_from!((1 << 7) - 1)),
                enable(meta) * (x16 - x18 * constant_from!(1 << 2) - x18_helper.clone()),
                enable(meta) * (x18_helper + x18_helper_diff - constant_from!((1 << 2) - 1)),
                enable(meta) * (x.clone() - x3 * constant_from!(1 << 3) - x3_helper.clone()),
                enable(meta) * (x3_helper + x3_helper_diff - constant_from!((1 << 3) - 1)),
                enable(meta)
                    * (curr!(meta, self.op)
                        - constant_from!(Sha256HelperOp::SSigma0 as i32 * (index * 8))),
                enable(meta)
                    * (self.op_code(meta)
                        - encode_op_expr(curr!(meta, self.op), [&x; OP_ARGS_NUM])),
            ]
        });
    }

    pub fn configure_in_table(
        &self,
        meta: &mut ConstraintSystem<F>,
        expr: impl FnOnce(&mut VirtualCells<'_, F>) -> Expression<F>,
    ) {
        meta.lookup_any("in sha helper", |meta| {
            vec![(
                expr(meta),
                fixed_curr!(meta, self.block_first_line_sel) * self.op_code(meta),
            )]
        });
    }
}
