use std::rc::Rc;
use crate::runtime::host::{host_env::HostEnv, ForeignContext};
use halo2_proofs::arithmetic::CurveAffine;
use halo2_proofs::pairing::bls12_381::{G1Affine, G2Affine,
    Gt as Bls381Gt,
    pairing,
};
use super::{
    bls381_fq_to_limbs,
    fetch_fq,
    fetch_fq2,
};
use zkwasm_host_circuits::host::ForeignInst;

#[derive(Default)]
struct BlsPairContext {
    pub limbs: Vec<u64>,
    pub g1_identity: bool,
    pub g2_identity: bool,
    pub result_limbs: Vec<u64>,
    pub result_cursor: usize,
    pub input_cursor: usize,
}

impl BlsPairContext {
    fn bls381_gt_to_limbs(&mut self, g: Bls381Gt) {
       bls381_fq_to_limbs(&mut self.result_limbs,g.0.c0.c0.c0);
       bls381_fq_to_limbs(&mut self.result_limbs,g.0.c0.c0.c1);
       bls381_fq_to_limbs(&mut self.result_limbs,g.0.c0.c1.c0);
       bls381_fq_to_limbs(&mut self.result_limbs,g.0.c0.c1.c1);
       bls381_fq_to_limbs(&mut self.result_limbs,g.0.c0.c2.c0);
       bls381_fq_to_limbs(&mut self.result_limbs,g.0.c0.c2.c1);
       bls381_fq_to_limbs(&mut self.result_limbs,g.0.c0.c0.c0);
       bls381_fq_to_limbs(&mut self.result_limbs,g.0.c0.c0.c1);
       bls381_fq_to_limbs(&mut self.result_limbs,g.0.c0.c1.c0);
       bls381_fq_to_limbs(&mut self.result_limbs,g.0.c0.c1.c1);
       bls381_fq_to_limbs(&mut self.result_limbs,g.0.c0.c2.c0);
       bls381_fq_to_limbs(&mut self.result_limbs,g.0.c0.c2.c1);
       bls381_fq_to_limbs(&mut self.result_limbs,g.0.c1.c0.c0);
       bls381_fq_to_limbs(&mut self.result_limbs,g.0.c1.c0.c1);
       bls381_fq_to_limbs(&mut self.result_limbs,g.0.c1.c1.c0);
       bls381_fq_to_limbs(&mut self.result_limbs,g.0.c1.c1.c1);
       bls381_fq_to_limbs(&mut self.result_limbs,g.0.c1.c2.c0);
       bls381_fq_to_limbs(&mut self.result_limbs,g.0.c1.c2.c1);
       bls381_fq_to_limbs(&mut self.result_limbs,g.0.c1.c0.c0);
       bls381_fq_to_limbs(&mut self.result_limbs,g.0.c1.c0.c1);
       bls381_fq_to_limbs(&mut self.result_limbs,g.0.c1.c1.c0);
       bls381_fq_to_limbs(&mut self.result_limbs,g.0.c1.c1.c1);
       bls381_fq_to_limbs(&mut self.result_limbs,g.0.c1.c2.c0);
       bls381_fq_to_limbs(&mut self.result_limbs,g.0.c1.c2.c1);
    }

}

impl ForeignContext for BlsPairContext {}

use specs::external_host_call_table::ExternalHostCallSignature;
pub fn register_blspair_foreign(env: &mut HostEnv) {
    let foreign_blspair_plugin = env
            .external_env
            .register_plugin("foreign_blspair", Box::new(BlsPairContext::default()));

    env.external_env.register_function(
        "blspair_g1",
        ForeignInst::BlsPairG1 as  usize,
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
        ForeignInst::BlsPairG2 as usize,
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
                        x: fetch_fq2(&context.limbs,2),
                        y: fetch_fq2(&context.limbs,4),
                        infinity: (0 as u8).into()
                        }
                    };
                    let ab = pairing(&g1, &g2);
                    log::debug!("gt {:?}", ab);
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
        ForeignInst::BlsPairG3 as usize,
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
