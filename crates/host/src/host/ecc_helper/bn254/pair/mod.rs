use delphinus_zkwasm::circuits::config::zkwasm_k;
use delphinus_zkwasm::runtime::host::host_env::HostEnv;
use delphinus_zkwasm::runtime::host::ForeignContext;
use delphinus_zkwasm::runtime::host::ForeignStatics;
use halo2_proofs::arithmetic::CurveAffine;
use halo2_proofs::pairing::bn256::pairing;
use halo2_proofs::pairing::bn256::G1Affine;
use halo2_proofs::pairing::bn256::G2Affine;
use halo2_proofs::pairing::bn256::Gt as BN254Gt;
use halo2_proofs::pairing::group::prime::PrimeCurveAffine;
use std::rc::Rc;
use wasmi::tracer::Observer;

use super::bn254_fq_to_limbs;
use super::fetch_fq;
use super::fetch_fq2;
use super::LIMBNB;

use zkwasm_host_circuits::circuits::bn256::Bn256PairChip;
use zkwasm_host_circuits::circuits::host::HostOpSelector;
use zkwasm_host_circuits::host::ForeignInst::Bn254PairG1;
use zkwasm_host_circuits::host::ForeignInst::Bn254PairG2;
use zkwasm_host_circuits::host::ForeignInst::Bn254PairG3;

#[derive(Default)]
struct BN254PairContext {
    pub limbs: Vec<u64>,
    pub g1_identity: bool,
    pub g2_identity: bool,
    pub gt: Option<BN254Gt>,
    pub result_limbs: Vec<u64>,
    pub result_cursor: usize,
    pub input_cursor: usize,
    pub used_round: usize,
}

impl BN254PairContext {
    fn bn254_gt_to_limbs(&mut self, g: BN254Gt) {
        bn254_fq_to_limbs(&mut self.result_limbs, g.0.c0.c0.c0);
        bn254_fq_to_limbs(&mut self.result_limbs, g.0.c0.c0.c1);
        bn254_fq_to_limbs(&mut self.result_limbs, g.0.c0.c1.c0);
        bn254_fq_to_limbs(&mut self.result_limbs, g.0.c0.c1.c1);
        bn254_fq_to_limbs(&mut self.result_limbs, g.0.c0.c2.c0);
        bn254_fq_to_limbs(&mut self.result_limbs, g.0.c0.c2.c1);
        bn254_fq_to_limbs(&mut self.result_limbs, g.0.c0.c0.c0);
        bn254_fq_to_limbs(&mut self.result_limbs, g.0.c0.c0.c1);
        bn254_fq_to_limbs(&mut self.result_limbs, g.0.c0.c1.c0);
        bn254_fq_to_limbs(&mut self.result_limbs, g.0.c0.c1.c1);
        bn254_fq_to_limbs(&mut self.result_limbs, g.0.c0.c2.c0);
        bn254_fq_to_limbs(&mut self.result_limbs, g.0.c0.c2.c1);
        bn254_fq_to_limbs(&mut self.result_limbs, g.0.c1.c0.c0);
        bn254_fq_to_limbs(&mut self.result_limbs, g.0.c1.c0.c1);
        bn254_fq_to_limbs(&mut self.result_limbs, g.0.c1.c1.c0);
        bn254_fq_to_limbs(&mut self.result_limbs, g.0.c1.c1.c1);
        bn254_fq_to_limbs(&mut self.result_limbs, g.0.c1.c2.c0);
        bn254_fq_to_limbs(&mut self.result_limbs, g.0.c1.c2.c1);
        bn254_fq_to_limbs(&mut self.result_limbs, g.0.c1.c0.c0);
        bn254_fq_to_limbs(&mut self.result_limbs, g.0.c1.c0.c1);
        bn254_fq_to_limbs(&mut self.result_limbs, g.0.c1.c1.c0);
        bn254_fq_to_limbs(&mut self.result_limbs, g.0.c1.c1.c1);
        bn254_fq_to_limbs(&mut self.result_limbs, g.0.c1.c2.c0);
        bn254_fq_to_limbs(&mut self.result_limbs, g.0.c1.c2.c1);
    }
}

impl ForeignContext for BN254PairContext {
    fn get_statics(&self) -> Option<ForeignStatics> {
        Some(ForeignStatics {
            used_round: self.used_round,
            max_round: Bn256PairChip::max_rounds(zkwasm_k() as usize),
        })
    }
}

use specs::external_host_call_table::ExternalHostCallSignature;
pub fn register_bn254pair_foreign(env: &mut HostEnv) {
    let foreign_blspair_plugin = env
        .external_env
        .register_plugin("foreign_blspair", Box::new(BN254PairContext::default()));

    env.external_env.register_function(
        "bn254pair_g1",
        Bn254PairG1 as usize,
        ExternalHostCallSignature::Argument,
        foreign_blspair_plugin.clone(),
        Rc::new(
            |_obs: &Observer, context: &mut dyn ForeignContext, args: wasmi::RuntimeArgs| {
                let context = context.downcast_mut::<BN254PairContext>().unwrap();
                if context.input_cursor == LIMBNB * 2 {
                    let t: u64 = args.nth(0);
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
        "bn254pair_g2",
        Bn254PairG2 as usize,
        ExternalHostCallSignature::Argument,
        foreign_blspair_plugin.clone(),
        Rc::new(
            |_obs: &Observer, context: &mut dyn ForeignContext, args: wasmi::RuntimeArgs| {
                let context = context.downcast_mut::<BN254PairContext>().unwrap();
                if context.input_cursor == LIMBNB * 4 {
                    let t: u64 = args.nth(0);
                    context.g2_identity = t != 0;
                    let g1 = if context.g1_identity {
                        G1Affine::identity()
                    } else {
                        let opt: Option<_> = G1Affine::from_xy(
                            fetch_fq(&context.limbs, 0),
                            fetch_fq(&context.limbs, 1),
                        )
                        .into();
                        opt.expect("invalid g1 affine")
                    };
                    let g2 = if context.g2_identity {
                        G2Affine::identity()
                    } else {
                        let opt: Option<_> = G2Affine {
                            x: fetch_fq2(&context.limbs, 2),
                            y: fetch_fq2(&context.limbs, 4),
                        }
                        .into();
                        opt.expect("invalid g2 affine")
                    };
                    context.input_cursor = 0;
                    context.limbs = vec![];
                    let ab = pairing(&g1, &g2);
                    context.gt = Some(context.gt.map_or_else(|| ab, |x| x + ab));
                    //log::debug!("\n\ngt is {:?}", context.gt);
                } else {
                    context.limbs.push(args.nth(0));
                    context.input_cursor += 1;
                };
                None
            },
        ),
    );

    env.external_env.register_function(
        "bn254pair_pop",
        Bn254PairG3 as usize,
        ExternalHostCallSignature::Return,
        foreign_blspair_plugin.clone(),
        Rc::new(
            |_obs: &Observer, context: &mut dyn ForeignContext, _args: wasmi::RuntimeArgs| {
                let context = context.downcast_mut::<BN254PairContext>().unwrap();
                if context.result_cursor == 0 {
                    let gt = context.gt.unwrap();
                    log::debug!("\n\ngt is {:?}", context.gt);
                    context.bn254_gt_to_limbs(gt);
                }
                let ret = Some(wasmi::RuntimeValue::I64(
                    context.result_limbs[context.result_cursor] as i64,
                ));
                context.result_cursor += 1;
                ret
            },
        ),
    );
}
