use std::rc::Rc;
use std::ops::Add;
use crate::runtime::host::{host_env::HostEnv, ForeignContext};
use halo2_proofs::pairing::bn256::G1Affine;
use halo2_proofs::pairing::group::prime::PrimeCurveAffine;
use zkwasm_host_circuits::host::ForeignInst::{
    Bn254SumNew, Bn254SumScalar, Bn254SumG1, Bn254SumResult,
};

use super::{
    bn254_fq_to_limbs,
    fetch_g1,
    fetch_fr,
};

struct BN254SumContext {
    pub acc: G1Affine,
    pub limbs: Vec<u64>,
    pub coeffs: Vec<u64>,
    pub result_limbs: Option<Vec<u64>>,
    pub result_cursor: usize,
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

    pub fn default() -> Self {
        BN254SumContext {
            acc: G1Affine::identity(),
            limbs: vec![],
            coeffs: vec![],
            result_limbs: None,
            result_cursor: 0,
        }
    }


    pub fn bn254_sum_new(&mut self, new: usize) {
        log::debug!("new bn254 sum context");
        self.result_limbs = None;
        self.result_cursor = 0;
        self.limbs = vec![];
        self.coeffs = vec![];
        if new != 0 {
            G1Affine::identity();
        }
    }


    fn bn254_sum_push_scalar(&mut self, v: u64) {
        log::debug!("push scalar {}", v);
        self.coeffs.push(v)
    }

    fn bn254_sum_push_limb(&mut self, v: u64) {
        log::debug!("push limb {}", v);
        self.limbs.push(v)
    }
}

impl ForeignContext for BN254SumContext {}

/*
 *   ForeignInst::Bn254SumNew
 *   ForeignInst::Bn254SumScalar
 *   ForeignInst::Bn254SumG1
 *   ForeignInst::Bn254SumResult
 */

use specs::external_host_call_table::ExternalHostCallSignature;
pub fn register_bn254sum_foreign(env: &mut HostEnv) {
    let foreign_bn254sum_plugin = env
            .external_env
            .register_plugin("foreign_bn254sum", Box::new(BN254SumContext::default()));

    env.external_env.register_function(
        "bn254_sum_new",
        Bn254SumNew as usize,
        ExternalHostCallSignature::Argument,
        foreign_bn254sum_plugin.clone(),
        Rc::new(
            |context: &mut dyn ForeignContext, args: wasmi::RuntimeArgs| {
                let context = context.downcast_mut::<BN254SumContext>().unwrap();
                context.bn254_sum_new(args.nth::<u64>(0) as usize);
                None
            },
        ),
    );


    env.external_env.register_function(
        "bn254_sum_scalar",
        Bn254SumScalar as usize,
        ExternalHostCallSignature::Argument,
        foreign_bn254sum_plugin.clone(),
        Rc::new(
            |context: &mut dyn ForeignContext, args: wasmi::RuntimeArgs| {
                let context = context.downcast_mut::<BN254SumContext>().unwrap();
                context.bn254_sum_push_scalar(args.nth::<u64>(0));
                None
            },
        ),
    );

    env.external_env.register_function(
        "bn254_sum_g1",
        Bn254SumG1 as usize,
        ExternalHostCallSignature::Argument,
        foreign_bn254sum_plugin.clone(),
        Rc::new(
            |context: &mut dyn ForeignContext, args: wasmi::RuntimeArgs| {
                let context = context.downcast_mut::<BN254SumContext>().unwrap();
                context.bn254_sum_push_limb(args.nth::<u64>(0));
                None
            },
        ),
    );


    env.external_env.register_function(
        "bn254_sum_finalize",
        Bn254SumResult as usize,
        ExternalHostCallSignature::Return,
        foreign_bn254sum_plugin.clone(),
        Rc::new(
            |context: &mut dyn ForeignContext, _args: wasmi::RuntimeArgs| {
                let context = context.downcast_mut::<BN254SumContext>().unwrap();
                log::debug!("calculate finalize");
                context.result_limbs.clone().map_or_else(
                    || {
                        let coeff = fetch_fr(&context.coeffs);
                        log::debug!("coeff is {:?}", coeff);
                        let g1 = fetch_g1(&context.limbs);
                        log::debug!("g1 is {:?}", g1);
                        let next = g1 * coeff;
                        let g1result = context.acc.add(next).into();
                        log::debug!("msm result: {:?}", g1result);
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
