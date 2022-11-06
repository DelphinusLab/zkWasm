use halo2_proofs::{arithmetic::FieldExt, plonk::Expression};

use crate::{constant, constant_from};

pub fn largrange_expr<F: FieldExt>(x: Expression<F>, set: Vec<u64>, when: u64) -> Expression<F> {
    let set: Vec<u64> = set.into_iter().filter(|v| *v != when).collect();

    let numerator = set
        .iter()
        .map(|kind| x.clone() - constant_from!(*kind))
        .fold(constant_from!(1), |r, v| r * v);
    let denominator = set
        .iter()
        .map(|kind| F::from(when) - F::from(*kind))
        .fold(F::from(1), |r, v| r * v);

    numerator * constant!(denominator.invert().unwrap())
}
