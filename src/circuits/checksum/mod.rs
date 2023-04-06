use halo2_proofs::arithmetic::FieldExt;
use halo2_proofs::circuit::AssignedCell;
use halo2_proofs::circuit::Layouter;
use halo2_proofs::plonk::ConstraintSystem;
use halo2_proofs::plonk::Error;

use self::poseidon::primitives::p128pow5t9::RATE;
use self::poseidon::primitives::p128pow5t9::WIDTH;
use self::poseidon::primitives::ConstantLength;
use self::poseidon::primitives::P128Pow5T9;
use self::poseidon::Hash;
use self::poseidon::Pow5Chip;
use self::poseidon::Pow5Config;

pub mod poseidon;

// image data: 8192
// frame table: 4
// event table: 1

pub const L: usize = 8192 + 4 + 1;

#[derive(Clone)]
pub(crate) struct CheckSumConfig<F: FieldExt> {
    pow5_config: Pow5Config<F, 9, 8>,
}

pub(crate) struct CheckSumChip<F: FieldExt> {
    config: CheckSumConfig<F>,
}

impl<F: FieldExt> CheckSumConfig<F> {
    pub(crate) fn configure(meta: &mut ConstraintSystem<F>) -> Self {
        let state = (0..WIDTH).map(|_| meta.advice_column()).collect::<Vec<_>>();
        let partial_sbox = meta.advice_column();
        let mid_0_helper = meta.advice_column();

        let rc_a = (0..WIDTH).map(|_| meta.fixed_column()).collect::<Vec<_>>();
        let rc_b = (0..WIDTH).map(|_| meta.fixed_column()).collect::<Vec<_>>();

        Self {
            pow5_config: Pow5Chip::configure::<P128Pow5T9<F>>(
                meta,
                state.try_into().unwrap(),
                partial_sbox,
                mid_0_helper,
                rc_a.try_into().unwrap(),
                rc_b.try_into().unwrap(),
            ),
        }
    }
}

impl<F: FieldExt> CheckSumChip<F> {
    pub(crate) fn new(config: CheckSumConfig<F>) -> Self {
        Self { config }
    }

    pub(crate) fn assign(
        &self,
        layouter: &mut impl Layouter<F>,
        message: Vec<AssignedCell<F, F>>,
    ) -> Result<AssignedCell<F, F>, Error> {
        let config = self.config.pow5_config.clone();
        let chip = Pow5Chip::construct(config.clone());

        assert_eq!(message.len(), L);

        let hasher = Hash::<_, _, P128Pow5T9<F>, ConstantLength<L>, WIDTH, RATE>::init(
            chip,
            layouter.namespace(|| "init"),
        )?;
        let output = hasher.hash(layouter.namespace(|| "hash"), message.try_into().unwrap())?;

        Ok(output)
    }
}
