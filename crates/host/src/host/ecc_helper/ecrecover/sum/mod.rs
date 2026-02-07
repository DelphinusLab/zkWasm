use delphinus_zkwasm::runtime::host::host_env::HostEnv;
use delphinus_zkwasm::runtime::host::ForeignContext;
use delphinus_zkwasm::runtime::host::ForeignStatics;
use std::rc::Rc;
use zkwasm_host_circuits::host::ForeignInst::EcRecoverFinalize;
use zkwasm_host_circuits::host::ForeignInst::EcRecoverNew;
use zkwasm_host_circuits::host::ForeignInst::EcRecoverPush;

use super::fetch_point;
use super::fetch_scalar;
use k256::elliptic_curve::ops::LinearCombination;
use k256::elliptic_curve::point::AffineCoordinates;
use k256::elliptic_curve::ScalarPrimitive;
use k256::AffinePoint;
use k256::NonZeroScalar;
use k256::ProjectivePoint;

const SIG_R_LIMBS: usize = 4;
const SIG_S_LIMBS: usize = 4;
const Z_LIMBS: usize = 4;
const R_POINT_LIMBS: usize = 5;

const SIG_R_END: usize = SIG_R_LIMBS;
const SIG_S_END: usize = SIG_R_END + SIG_S_LIMBS;
const Z_END: usize = SIG_S_END + Z_LIMBS;
const R_POINT_END: usize = Z_END + R_POINT_LIMBS;

struct EcRecoverContext {
    pub sig_r: Vec<u64>,
    pub sig_s: Vec<u64>,
    pub hash_z: Vec<u64>,
    //x:[u64;4],is_y_odd:u64
    pub compressed_r_point: Vec<u64>,
    pub input_cursor: usize,
    pub result_limbs: Option<Vec<u64>>,
    pub result_cursor: usize,
    pub used_round: usize,
}

impl Default for EcRecoverContext {
    fn default() -> Self {
        Self {
            sig_r: vec![],
            sig_s: vec![],
            hash_z: vec![],
            compressed_r_point: vec![],
            input_cursor: 0,
            result_limbs: None,
            result_cursor: 0,
            used_round: 0,
        }
    }
}

impl EcRecoverContext {
    fn ecrecover_result_to_limbs(&mut self, g: AffinePoint) {
        let mut limbs = vec![];
        let x_bytes = g.x();
        let x_scalar = ScalarPrimitive::<k256::Secp256k1>::from_bytes(&x_bytes).unwrap();
        limbs.extend(x_scalar.as_limbs().iter().map(|v| v.0).collect::<Vec<_>>());
        limbs.push(bool::from(g.y_is_odd()) as u64);
        self.result_limbs = Some(limbs);
    }

    pub fn ecrecover_new(&mut self, _new: usize) {
        self.result_limbs = None;
        self.result_cursor = 0;
        self.compressed_r_point = vec![];
        self.sig_r = vec![];
        self.sig_s = vec![];
        self.hash_z = vec![];
        self.used_round += 1;
        self.input_cursor = 0;
    }

    // Input layout:
    // | sig_r (4) | sig_s (4) | hash_z (4) | compressed_r_point (5) |
    fn ecrecover_push(&mut self, v: u64) {
        let i = self.input_cursor;
        if i < SIG_R_END {
            self.sig_r.push(v);
        } else if i < SIG_S_END {
            self.sig_s.push(v);
        } else if i < Z_END {
            self.hash_z.push(v);
        } else if i < R_POINT_END {
            self.compressed_r_point.push(v);
        }

        self.input_cursor += 1;
    }
}

impl ForeignContext for EcRecoverContext {
    fn get_statics(&self, _k: u32) -> Option<ForeignStatics> {
        Some(ForeignStatics {
            used_round: self.used_round,
            // max_round: EcRecoverChip::max_rounds(k as usize),
            //TODO Derive the maximum number of rounds from the actual execution
            // traces of the ecrecover host at k.
            max_round: 50,
        })
    }
}

use specs::external_host_call_table::ExternalHostCallSignature;
pub fn register_ecrecover_foreign(env: &mut HostEnv) {
    let foreign_ecrecover_plugin = env
        .external_env
        .register_plugin("foreign_ecrecover", Box::<EcRecoverContext>::default());

    env.external_env.register_function(
        "ecrecover_new",
        EcRecoverNew as usize,
        ExternalHostCallSignature::Argument,
        foreign_ecrecover_plugin.clone(),
        Rc::new(
            |_obs, context: &mut dyn ForeignContext, args: wasmi::RuntimeArgs| {
                let context = context.downcast_mut::<EcRecoverContext>().unwrap();
                context.ecrecover_new(args.nth::<u64>(0) as usize);
                None
            },
        ),
    );

    env.external_env.register_function(
        "ecrecover_push",
        EcRecoverPush as usize,
        ExternalHostCallSignature::Argument,
        foreign_ecrecover_plugin.clone(),
        Rc::new(
            |_obs, context: &mut dyn ForeignContext, args: wasmi::RuntimeArgs| {
                let context = context.downcast_mut::<EcRecoverContext>().unwrap();
                context.ecrecover_push(args.nth::<u64>(0));
                None
            },
        ),
    );

    env.external_env.register_function(
        "ecrecover_finalize",
        EcRecoverFinalize as usize,
        ExternalHostCallSignature::Return,
        foreign_ecrecover_plugin,
        Rc::new(
            |_obs, context: &mut dyn ForeignContext, _args: wasmi::RuntimeArgs| {
                let context = context.downcast_mut::<EcRecoverContext>().unwrap();
                log::debug!("calculate finalize");
                context.result_limbs.clone().map_or_else(
                    || {
                        let sig_r = fetch_scalar(&context.sig_r);
                        let sig_s = fetch_scalar(&context.sig_s);
                        let hash_z = fetch_scalar(&context.hash_z);
                        let r_point = fetch_point(&context.compressed_r_point);

                        let r = NonZeroScalar::from_uint(sig_r).unwrap();
                        let s = NonZeroScalar::from_uint(sig_s).unwrap();
                        let z = NonZeroScalar::from_uint(hash_z).unwrap();

                        let r_point = ProjectivePoint::from(r_point);
                        let r_inv = NonZeroScalar::new(r.invert().unwrap()).unwrap();
                        let u1 = -(r_inv * z);
                        let u2 = r_inv * s;
                        let pk: AffinePoint = ProjectivePoint::lincomb(
                            &ProjectivePoint::GENERATOR,
                            &u1,
                            &r_point,
                            &u2,
                        )
                        .into();
                        context.ecrecover_result_to_limbs(pk);
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
