use std::marker::PhantomData;

use halo2_proofs::{
    arithmetic::FieldExt,
    plonk::{Advice, Column, ConstraintSystem, Fixed},
};

use crate::{constant_from, curr, fixed_curr, nextn};

use super::rtable::RangeTableConfig;

pub struct BitTableConfig<F: FieldExt> {
    step_sel: Column<Fixed>,
    lookup_sel: Column<Fixed>,
    value: Column<Advice>,
    _mark: PhantomData<F>,
}
/*
| step_sel  | lookup_sel |   val      |
|    1      |     0      |  encode    |
|    0      |     1      |  op        |
|    0      |     0      |  l_u8_0    |
|    0      |     0      |  r_u8_0    |
|    0      |     0      |  res_u8_0  |
|    0      |     1      |  op        |
|    0      |     0      |  l_u8_1    |
|    0      |     0      |  r_u8_1    |
|    0      |     0      |  res_u8_1  |
...
|    0      |     1      |  op        |
|    0      |     0      |  l_u8_7    |
|    0      |     0      |  r_u8_7    |
|    0      |     0      |  res_u8_7  |

|    1      |     0      |  encode    |
|    0      |     1      |  op        |
*/
impl<F: FieldExt> BitTableConfig<F> {
    pub(crate) fn configure(meta: &mut ConstraintSystem<F>, rtable: RangeTableConfig<F>) -> Self {
        let step_sel = meta.fixed_column();
        let lookup_sel = meta.fixed_column();
        let value = meta.advice_column();

        rtable.configure_in_u8_range(meta, "bit table u8", |meta| {
            (constant_from!(1) - fixed_curr!(meta, step_sel)) * curr!(meta, value)
        });

        rtable.configure_in_u8_bit_table(
            meta,
            "bit table u8 bit table lookup",
            |meta| curr!(meta, value),
            |meta| nextn!(meta, value, 1),
            |meta| nextn!(meta, value, 2),
            |meta| nextn!(meta, value, 3),
            |meta| fixed_curr!(meta, lookup_sel),
        );

        meta.create_gate("bit table encode", |meta| {
            vec![
                (
                    curr!(meta, value) - 
                ) * fixed_curr!(meta, step_sel)
            ]
        });

        Self {
            step_sel,
            lookup_sel,
            value,
            _mark: PhantomData,
        }
    }
}
