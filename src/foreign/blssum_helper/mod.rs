use std::ops::{AddAssign, Add};
use std::rc::Rc;
use crate::runtime::host::{host_env::HostEnv, ForeignContext};
use ark_std::Zero;
use halo2_proofs::arithmetic::{BaseExt, CurveAffine};
use halo2_proofs::pairing::bls12_381::{G1Affine, Fq as Bls381Fq};
use num_bigint::BigUint;


const BLSSUM_G1:usize= 3;
const BLSSUM_RESULT:usize= 4;

pub fn bn_to_field<F: BaseExt>(bn: &BigUint) -> F {
    let mut bytes = bn.to_bytes_le();
    bytes.resize(48, 0);
    let mut bytes = &bytes[..];
    F::read(&mut bytes).unwrap()
}

pub fn field_to_bn<F: BaseExt>(f: &F) -> BigUint {
    let mut bytes: Vec<u8> = Vec::new();
    f.write(&mut bytes).unwrap();
    BigUint::from_bytes_le(&bytes[..])
}


#[derive(Default)]
struct BlsSumContext {
    pub limbs: Vec<u64>,
    pub g1_identity: Vec<bool>,
    pub result_limbs: Option<Vec<u64>>,
    pub result_cursor: usize,
    pub input_cursor: usize,
}

fn fetch_fq(limbs: &Vec<u64>, index:usize) -> Bls381Fq {
    let mut bn = BigUint::zero();
    for i in 0..8 {
        bn.add_assign(BigUint::from_u64(limbs[index * 8 + i]).unwrap() << (i * 54))
    }
    bn_to_field(&bn)
}

fn fetch_g1(limbs: &Vec<u64>, g1_identity: bool) -> G1Affine {
    if g1_identity {
        G1Affine::identity()
    } else {
        let opt:Option<_> = G1Affine::from_xy(
            fetch_fq(limbs,0),
            fetch_fq(limbs,1)
        ).into();
        opt.expect("from xy failed, not on curve")
    }
}

impl BlsSumContext {
    fn bls381_fq_to_limbs(&mut self, f: Bls381Fq) {
        let mut bn = field_to_bn(&f);
        for _ in 0..8 {
            let d:BigUint = BigUint::from(1u64 << 54);
            let r = bn.clone() % d.clone();
            let value = if r == BigUint::from(0 as u32) {
                0 as u64
            } else {
                r.to_u64_digits()[0]
            };
            bn = bn / d;
            self.result_limbs.as_mut().unwrap().append(&mut vec![value]);
        };
    }
    fn bls381_result_to_limbs(&mut self, g: G1Affine) {
        self.result_limbs = Some (vec![]);
        self.bls381_fq_to_limbs(g.x);
        self.bls381_fq_to_limbs(g.y);
        if g.is_identity().into() {
            self.result_limbs.as_mut().unwrap().append(&mut vec![1u64]);
        } else {
            self.result_limbs.as_mut().unwrap().append(&mut vec![0u64]);
        }
    }
}

impl ForeignContext for BlsSumContext {}

use num_traits::FromPrimitive;
use specs::external_host_call_table::ExternalHostCallSignature;
pub fn register_blssum_foreign(env: &mut HostEnv) {
    let foreign_blssum_plugin = env
            .external_env
            .register_plugin("foreign_blssum", Box::new(BlsSumContext::default()));

    env.external_env.register_function(
        "blssum_g1",
        BLSSUM_G1,
        ExternalHostCallSignature::Argument,
        foreign_blssum_plugin.clone(),
        Rc::new(
            |context: &mut dyn ForeignContext, args: wasmi::RuntimeArgs| {
                let context = context.downcast_mut::<BlsSumContext>().unwrap();
                if context.input_cursor == 16 {
                    let t:u64 = args.nth(0);
                    context.g1_identity.push(t != 0);
                    context.input_cursor = 0;
                } else {
                    context.limbs.push(args.nth(0));
                    context.input_cursor += 1;
                }
                None
            },
        ),
    );

    env.external_env.register_function(
        "blssum_pop",
        BLSSUM_RESULT,
        ExternalHostCallSignature::Return,
        foreign_blssum_plugin.clone(),
        Rc::new(
            |context: &mut dyn ForeignContext, _args: wasmi::RuntimeArgs| {
                let context = context.downcast_mut::<BlsSumContext>().unwrap();
                context.result_limbs.clone().map_or_else(
                    || {
                        let fqs = context.limbs.chunks(16).zip(context.g1_identity.clone()).map(|(limbs, identity)| {
                            fetch_g1(&limbs.to_vec(), identity)
                        }).collect::<Vec<G1Affine>>();
                        let g1result = fqs[1..fqs.len()].into_iter().fold(fqs[0], |acc:G1Affine, x| {
                            let acc = acc.add(x.clone()).into();
                            acc
                        });
                        context.bls381_result_to_limbs(g1result);
                    },
                    |_| {()}
                );
                let limbs = context.result_limbs.clone().unwrap();
                let ret = Some(wasmi::RuntimeValue::I64(limbs[context.result_cursor] as i64));
                context.result_cursor += 1;
                ret
            },
        ),
    );
}
