use std::ops::{AddAssign, Shl};
use std::rc::Rc;
use crate::runtime::host::{host_env::HostEnv, ForeignContext};
use ark_std::Zero;
use halo2_proofs::arithmetic::{BaseExt, CurveAffine};
use halo2_proofs::pairing::bls12_381::{G1Affine, G2Affine,
    Fp2 as Bls381Fq2,
    Gt as Bls381Gt,
    Fq as Bls381Fq,
    pairing,
};
use num_bigint::BigUint;

const BLSPAIR_G1:usize= 0;
const BLSPAIR_G2:usize= 1;
const BLSPAIR_G3:usize= 2;

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
struct BlsPairContext {
    pub limbs: Vec<u64>,
    pub g1_identity: bool,
    pub g2_identity: bool,
    pub result_limbs: Vec<u64>,
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

impl BlsPairContext {
    fn fetch_fq2(&self, index:usize) -> Bls381Fq2 {
        Bls381Fq2 {
            c0: fetch_fq(&self.limbs,index),
            c1: fetch_fq(&self.limbs, index+1),
        }
    }
    fn bls381_fq_to_limbs(&mut self, f: Bls381Fq) {
        let mut bn = field_to_bn(&f);
        for _ in 0..8 {
            let d:BigUint = BigUint::from(2 as u64).shl(54);
            let r = bn.clone() % d.clone();
            let value = if r == BigUint::from(0 as u32) {
                0 as u64
            } else {
                r.to_u64_digits()[0]
            };
            bn = bn / d;
            self.result_limbs.append(&mut vec![value]);
        };
    }
    fn bls381_gt_to_limbs(&mut self, g: Bls381Gt) {
       self.bls381_fq_to_limbs(g.0.c0.c0.c0);
       self.bls381_fq_to_limbs(g.0.c0.c0.c1);
       self.bls381_fq_to_limbs(g.0.c0.c1.c0);
       self.bls381_fq_to_limbs(g.0.c0.c1.c1);
       self.bls381_fq_to_limbs(g.0.c0.c2.c0);
       self.bls381_fq_to_limbs(g.0.c0.c2.c1);
       self.bls381_fq_to_limbs(g.0.c0.c0.c0);
       self.bls381_fq_to_limbs(g.0.c0.c0.c1);
       self.bls381_fq_to_limbs(g.0.c0.c1.c0);
       self.bls381_fq_to_limbs(g.0.c0.c1.c1);
       self.bls381_fq_to_limbs(g.0.c0.c2.c0);
       self.bls381_fq_to_limbs(g.0.c0.c2.c1);
       self.bls381_fq_to_limbs(g.0.c1.c0.c0);
       self.bls381_fq_to_limbs(g.0.c1.c0.c1);
       self.bls381_fq_to_limbs(g.0.c1.c1.c0);
       self.bls381_fq_to_limbs(g.0.c1.c1.c1);
       self.bls381_fq_to_limbs(g.0.c1.c2.c0);
       self.bls381_fq_to_limbs(g.0.c1.c2.c1);
       self.bls381_fq_to_limbs(g.0.c1.c0.c0);
       self.bls381_fq_to_limbs(g.0.c1.c0.c1);
       self.bls381_fq_to_limbs(g.0.c1.c1.c0);
       self.bls381_fq_to_limbs(g.0.c1.c1.c1);
       self.bls381_fq_to_limbs(g.0.c1.c2.c0);
       self.bls381_fq_to_limbs(g.0.c1.c2.c1);
    }

}

impl ForeignContext for BlsPairContext {}

use num_traits::FromPrimitive;
use specs::external_host_call_table::ExternalHostCallSignature;
pub fn register_blspair_foreign(env: &mut HostEnv) {
    let foreign_blspair_plugin = env
            .external_env
            .register_plugin("foreign_blspair", Box::new(BlsPairContext::default()));

    env.external_env.register_function(
        "blspair_g1",
        BLSPAIR_G1,
        ExternalHostCallSignature::Argument,
        foreign_blspair_plugin.clone(),
        Rc::new(
            |context: &mut dyn ForeignContext, args: wasmi::RuntimeArgs| {
                let context = context.downcast_mut::<BlsPairContext>().unwrap();
                if context.input_cursor == 16 {
                    let t:u64 = args.nth(0);
                    context.g1_identity = t != 0;
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
        "blspair_g2",
        BLSPAIR_G2,
        ExternalHostCallSignature::Argument,
        foreign_blspair_plugin.clone(),
        Rc::new(
            |context: &mut dyn ForeignContext, args: wasmi::RuntimeArgs| {
                let context = context.downcast_mut::<BlsPairContext>().unwrap();
                if context.input_cursor == 32 {
                    let t:u64 = args.nth(0);
                    context.g2_identity = t !=0;
                    let g1 = if context.g1_identity {
                        G1Affine::identity()
                    } else {
                        G1Affine::from_xy(
                            fetch_fq(&context.limbs, 0),
                            fetch_fq(&context.limbs, 1)
                        ).unwrap()
                    };
                    let g2 = if context.g2_identity{
                        G2Affine::identity()
                    } else {
                        G2Affine {
                        x: context.fetch_fq2(2),
                        y: context.fetch_fq2(4),
                        infinity: (0 as u8).into()
                        }
                    };
                    let ab = pairing(&g1, &g2);
                    context.bls381_gt_to_limbs(ab);
                } else {
                    context.limbs.push(args.nth(0));
                    context.input_cursor += 1;
                };
                None
            },
        ),
    );

    env.external_env.register_function(
        "blspair_pop",
        BLSPAIR_G3,
        ExternalHostCallSignature::Return,
        foreign_blspair_plugin.clone(),
        Rc::new(
            |context: &mut dyn ForeignContext, _args: wasmi::RuntimeArgs| {
                let context = context.downcast_mut::<BlsPairContext>().unwrap();
                let ret = Some(wasmi::RuntimeValue::I64(context.result_limbs[context.result_cursor] as i64));
                context.result_cursor += 1;
                ret
            },
        ),
    );
}
