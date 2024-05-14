use super::babyjubjub_fq_to_limbs;
use super::fetch_g1;
use super::LIMBNB;
use num_bigint::BigUint;
use zkwasm_host_circuits::host::jubjub;

fn fetch_biguint(_limbs: &Vec<u64>) -> BigUint {
    BigUint::from_bytes_le(
        _limbs
            .iter()
            .map(|x| x.to_le_bytes())
            .flatten()
            .collect::<Vec<_>>()
            .as_slice(),
    )
}

pub struct BabyJubjubSumContext {
    pub k: u32,
    pub acc: jubjub::Point,
    pub limbs: Vec<u64>,
    pub coeffs: Vec<u64>,
    pub result_limbs: Option<Vec<u64>>,
    pub result_cursor: usize,
    pub input_cursor: usize,
    pub used_round: usize,
}

impl BabyJubjubSumContext {
    pub fn default(k: u32) -> Self {
        BabyJubjubSumContext {
            k,
            acc: jubjub::Point::identity(),
            limbs: vec![],
            coeffs: vec![],
            result_limbs: None,
            result_cursor: 0,
            input_cursor: 0,
            used_round: 0,
        }
    }

    pub fn babyjubjub_sum_new(&mut self, new: usize) {
        self.result_limbs = None;
        self.result_cursor = 0;
        self.limbs = vec![];
        self.input_cursor = 0;
        self.coeffs = vec![];
        self.used_round += 1;
        if new != 0 {
            self.acc = jubjub::Point::identity();
        }
    }

    pub fn babyjubjub_sum_push(&mut self, v: u64) {
        if self.input_cursor < LIMBNB * 2 {
            self.limbs.push(v);
            self.input_cursor += 1;
        } else if self.input_cursor < LIMBNB * 2 + 4 {
            self.coeffs.push(v);
            self.input_cursor += 1;
            if self.input_cursor == LIMBNB * 2 + 4 {
                self.input_cursor = 0;
            }
        }
    }

    pub fn babyjubjub_sum_finalize(&mut self) -> u64 {
        let limbs = self.result_limbs.clone();
        match limbs {
            None => {
                assert!(self.limbs.len() == LIMBNB * 2);
                let coeff = fetch_biguint(&self.coeffs.to_vec());
                let g1 = fetch_g1(&self.limbs.to_vec());
                log::debug!("acc is {:?}", self.acc);
                log::debug!("g1 is {:?}", g1);
                log::debug!("coeff is {:?} {}", coeff, self.coeffs.len());
                self.acc = self
                    .acc
                    .projective()
                    .add(&g1.mul_scalar(&coeff).projective())
                    .affine();
                log::debug!("msm result: {:?}", self.acc);
                self.babyjubjub_result_to_limbs(self.acc.clone());
            }
            _ => (),
        };
        let ret = self.result_limbs.as_ref().unwrap()[self.result_cursor];
        self.result_cursor += 1;

        ret
    }
}

impl BabyJubjubSumContext {
    fn babyjubjub_result_to_limbs(&mut self, g: jubjub::Point) {
        let mut limbs = vec![];
        babyjubjub_fq_to_limbs(&mut limbs, g.x);
        babyjubjub_fq_to_limbs(&mut limbs, g.y);
        self.result_limbs = Some(limbs);
    }
}
