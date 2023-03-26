use std::ops::Add;
use std::rc::Rc;
use crate::runtime::host::{host_env::HostEnv, ForeignContext};
use halo2_proofs::pairing::bn256::{Fr, G1Affine};
use halo2_proofs::pairing::group::prime::PrimeCurveAffine;

use super::{
    LIMBNB, BN254SUM_G1, BN254SUM_RESULT,
    bn254_fq_to_limbs,
    fetch_g1,
};

fn fetch_fr(_limbs: &Vec<u64>) -> Fr {
    //todo!();
    Fr::one()
}



#[derive(Default)]
struct BN254SumContext {
    pub limbs: Vec<u64>,
    pub coeffs: Vec<u64>,
    pub g1_identity: Vec<bool>,
    pub result_limbs: Option<Vec<u64>>,
    pub result_cursor: usize,
    pub input_cursor: usize,
}


impl BN254SumContext {
    fn bn254_result_to_limbs(&mut self, g: G1Affine) {
        let mut limbs = vec![];
        bn254_fq_to_limbs(&mut limbs, g.x);
        bn254_fq_to_limbs(&mut limbs, g.y);
        self.result_limbs = Some (limbs); 
        if g.is_identity().into() {
            self.result_limbs.as_mut().unwrap().append(&mut vec![1u64]);
        } else {
            self.result_limbs.as_mut().unwrap().append(&mut vec![0u64]);
        }
    }
}

impl ForeignContext for BN254SumContext {}

use specs::external_host_call_table::ExternalHostCallSignature;
pub fn register_bn254sum_foreign(env: &mut HostEnv) {
    let foreign_bn254sum_plugin = env
            .external_env
            .register_plugin("foreign_bn254sum", Box::new(BN254SumContext::default()));

    env.external_env.register_function(
        "bn254msm_g1",
        BN254SUM_G1,
        ExternalHostCallSignature::Argument,
        foreign_bn254sum_plugin.clone(),
        Rc::new(
            |context: &mut dyn ForeignContext, args: wasmi::RuntimeArgs| {
                let context = context.downcast_mut::<BN254SumContext>().unwrap();
                if context.input_cursor < LIMBNB*2  {
                    context.limbs.push(args.nth(0));
                    context.input_cursor += 1;
                } else if context.input_cursor == LIMBNB*2  {
                    let t:u64 = args.nth(0);
                    context.g1_identity.push(t != 0);
                    context.input_cursor += 1;
                } else if context.input_cursor == LIMBNB*2 + 4  {
                    context.coeffs.push(args.nth(0));
                    context.input_cursor = 0;
                } else {
                    context.coeffs.push(args.nth(0));
                    context.input_cursor += 1;
                }
                None
            },
        ),
    );

    env.external_env.register_function(
        "bn254msm_pop",
        BN254SUM_RESULT,
        ExternalHostCallSignature::Return,
        foreign_bn254sum_plugin.clone(),
        Rc::new(
            |context: &mut dyn ForeignContext, _args: wasmi::RuntimeArgs| {
                let context = context.downcast_mut::<BN254SumContext>().unwrap();
                context.result_limbs.clone().map_or_else(
                    || {
                        let fqs = context.limbs.chunks(LIMBNB*2)
                            .zip(context.coeffs.chunks(4))
                            .zip(context.g1_identity.clone()).map(|((limbs, coeffs), identity)| {
                            let coeff = fetch_fr(&coeffs.to_vec());
                            let g1 = fetch_g1(&limbs.to_vec(), identity);
                            //println!("coeff is {:?}", coeff);
                            (g1 * coeff).into()
                        }).collect::<Vec<G1Affine>>();
                        let g1result = fqs[1..fqs.len()].into_iter().fold(fqs[0], |acc:G1Affine, x| {
                            let acc = acc.add(x.clone()).into();
                            acc
                        });
                        //println!("msm result: {:?}", g1result);
                        context.bn254_result_to_limbs(g1result);
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
