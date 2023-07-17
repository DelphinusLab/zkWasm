use std::rc::Rc;
use crate::runtime::host::{host_env::HostEnv, ForeignContext};
use sha2::Digest;
use zkwasm_host_circuits::host::ForeignInst::{
    SHA256New,
    SHA256Push,
    SHA256Finalize,
};

use sha2::Sha256;

struct Generator {
    pub cursor: usize,
    pub values: Vec<u64>,
}

impl Generator {
    fn gen(&mut self) -> u64 {
        let r = self.values[self.cursor];
        self.cursor += 1;
        r
    }
}


struct Sha256Context {
    pub hasher: Option<Sha256>,
    pub generator: Generator,
    pub size: usize,
}

impl Sha256Context {
    fn default() -> Self {
        Sha256Context {
            hasher: None,
            generator: Generator {
                cursor: 0,
                values: vec![],
            },
            size: 0,
        }
    }
}


impl ForeignContext for Sha256Context {}

use specs::external_host_call_table::ExternalHostCallSignature;
pub fn register_sha256_foreign(env: &mut HostEnv) {
    let foreign_sha256_plugin = env
            .external_env
            .register_plugin("foreign_sh256", Box::new(Sha256Context::default()));

    env.external_env.register_function(
        "sha256_new",
        SHA256New as usize,
        ExternalHostCallSignature::Argument,
        foreign_sha256_plugin.clone(),
        Rc::new(
            |context: &mut dyn ForeignContext, args: wasmi::RuntimeArgs| {
                let context = context.downcast_mut::<Sha256Context>().unwrap();
                let hasher = context.hasher.as_mut().map_or({
                    Some(Sha256::new())
                }, |_| {
                    None
                });
                hasher.map(|s| {
                    context.hasher = Some(s);
                    context.size = args.nth::<u64>(0) as usize;
                });
                None
            },
        ),
    );

    env.external_env.register_function(
        "sha256_push",
        SHA256Push as usize,
        ExternalHostCallSignature::Argument,
        foreign_sha256_plugin.clone(),
        Rc::new(
            |context: &mut dyn ForeignContext, args: wasmi::RuntimeArgs| {
                let context = context.downcast_mut::<Sha256Context>().unwrap();
                context.hasher.as_mut().map(|s| {
                    let sz =  if context.size > 8 {
                        context.size -= 8;
                        8
                    } else {
                        let s = context.size;
                        context.size = 0;
                        s
                    };
                    let mut r = (args.nth::<u64>(0) as u64).to_le_bytes().to_vec();
                    r.truncate(sz);
                    s.update(r);
                });
                None
            },
        ),
    );


    env.external_env.register_function(
        "sha256_finalize",
        SHA256Finalize as usize,
        ExternalHostCallSignature::Return,
        foreign_sha256_plugin.clone(),
        Rc::new(
            |context: &mut dyn ForeignContext, _args: wasmi::RuntimeArgs| {
                let context = context.downcast_mut::<Sha256Context>().unwrap();
                context.hasher.as_ref().map(|s| {
                    let dwords:Vec<u8> = s.clone().finalize()[..].to_vec();
                    context.generator.values = dwords.chunks(8).map(|x| {
                        u64::from_le_bytes(x.to_vec().try_into().unwrap())
                    }).collect::<Vec<u64>>();
                });
                context.hasher = None;
                Some(wasmi::RuntimeValue::I64(context.generator.gen() as i64))
            },
        ),
    );
}
