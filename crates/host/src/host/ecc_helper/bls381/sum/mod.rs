use delphinus_zkwasm::circuits::config::zkwasm_k;
use delphinus_zkwasm::runtime::host::host_env::HostEnv;
use delphinus_zkwasm::runtime::host::ForeignContext;
use delphinus_zkwasm::runtime::host::ForeignStatics;
use halo2_proofs::pairing::bls12_381::G1Affine;
use std::ops::Add;
use std::rc::Rc;
use wasmi::tracer::Observer;

use super::bls381_fq_to_limbs;
use super::fetch_g1;

use zkwasm_host_circuits::circuits::bls::Bls381SumChip;
use zkwasm_host_circuits::circuits::host::HostOpSelector;
use zkwasm_host_circuits::host::ForeignInst;

#[derive(Default)]
struct BlsSumContext {
    pub limbs: Vec<u64>,
    pub g1_identity: Vec<bool>,
    pub result_limbs: Option<Vec<u64>>,
    pub result_cursor: usize,
    pub input_cursor: usize,
    pub used_round: usize,
}

impl BlsSumContext {
    fn bls381_result_to_limbs(&mut self, g: G1Affine) {
        let mut limbs = vec![];
        bls381_fq_to_limbs(&mut limbs, g.x);
        bls381_fq_to_limbs(&mut limbs, g.y);
        self.result_limbs = Some(limbs);
        if g.is_identity().into() {
            self.result_limbs.as_mut().unwrap().append(&mut vec![1u64]);
        } else {
            self.result_limbs.as_mut().unwrap().append(&mut vec![0u64]);
        }
    }
}

impl ForeignContext for BlsSumContext {
    fn get_statics(&self) -> Option<ForeignStatics> {
        Some(ForeignStatics {
            used_round: self.used_round,
            max_round: Bls381SumChip::max_rounds(zkwasm_k() as usize),
        })
    }
}

use specs::external_host_call_table::ExternalHostCallSignature;
pub fn register_blssum_foreign(env: &mut HostEnv) {
    let foreign_blssum_plugin = env
        .external_env
        .register_plugin("foreign_blssum", Box::new(BlsSumContext::default()));

    env.external_env.register_function(
        "blssum_g1",
        ForeignInst::BlsSumG1 as usize,
        ExternalHostCallSignature::Argument,
        foreign_blssum_plugin.clone(),
        Rc::new(
            |_obs: &Observer, context: &mut dyn ForeignContext, args: wasmi::RuntimeArgs| {
                let context = context.downcast_mut::<BlsSumContext>().unwrap();
                if context.input_cursor == 16 {
                    let t: u64 = args.nth(0);
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
        ForeignInst::BlsSumResult as usize,
        ExternalHostCallSignature::Return,
        foreign_blssum_plugin.clone(),
        Rc::new(
            |_obs: &Observer, context: &mut dyn ForeignContext, _args: wasmi::RuntimeArgs| {
                let context = context.downcast_mut::<BlsSumContext>().unwrap();
                context.result_limbs.clone().map_or_else(
                    || {
                        let fqs = context
                            .limbs
                            .chunks(16)
                            .zip(context.g1_identity.clone())
                            .map(|(limbs, identity)| fetch_g1(&limbs.to_vec(), identity))
                            .collect::<Vec<G1Affine>>();
                        let g1result =
                            fqs[1..fqs.len()]
                                .into_iter()
                                .fold(fqs[0], |acc: G1Affine, x| {
                                    let acc = acc.add(x.clone()).into();
                                    acc
                                });
                        context.bls381_result_to_limbs(g1result);
                    },
                    |_| (),
                );
                let limbs = context.result_limbs.clone().unwrap();
                let ret = Some(wasmi::RuntimeValue::I64(
                    limbs[context.result_cursor] as i64,
                ));
                context.result_cursor += 1;
                ret
            },
        ),
    );
}
