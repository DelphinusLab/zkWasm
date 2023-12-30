use delphinus_zkwasm::circuits::config::zkwasm_k;
use delphinus_zkwasm::runtime::host::host_env::HostEnv;
use delphinus_zkwasm::runtime::host::ForeignContext;
use delphinus_zkwasm::runtime::host::ForeignStatics;
use std::rc::Rc;
use wasmi::tracer::Observer;
use zkwasm_host_circuits::circuits::host::HostOpSelector;
use zkwasm_host_circuits::circuits::keccak256::KeccakChip;
use zkwasm_host_circuits::host::keccak256::Keccak;
use zkwasm_host_circuits::host::ForeignInst::Keccak256Finalize;
use zkwasm_host_circuits::host::ForeignInst::Keccak256New;
use zkwasm_host_circuits::host::ForeignInst::Keccak256Push;

pub use zkwasm_host_circuits::host::keccak256::KECCAK_HASHER;

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

struct Keccak256Context {
    pub hasher: Option<Keccak>,
    pub generator: Generator,
    pub buf: Vec<u64>,
    pub used_round: usize,
}

impl Keccak256Context {
    fn default() -> Self {
        Keccak256Context {
            hasher: None,
            generator: Generator {
                cursor: 0,
                values: vec![],
            },
            buf: vec![],
            used_round: 0,
        }
    }

    pub fn keccak_new(&mut self, new: usize) {
        self.buf = vec![];
        self.generator.cursor = 0;
        if new != 0 {
            self.hasher = Some(KECCAK_HASHER.clone());
            self.used_round += 1;
        }
    }

    pub fn keccak_push(&mut self, v: u64) {
        self.buf.push(v);
    }

    pub fn keccak_finalize(&mut self) -> u64 {
        assert!(self.buf.len() == 17);
        if self.generator.cursor == 0 {
            self.hasher.as_mut().map(|s| {
                log::debug!("perform hash with {:?}", self.buf);
                let r = s.update_exact(&self.buf.clone().try_into().unwrap());
                self.generator.values = r.to_vec();
            });
        }
        self.generator.gen()
    }
}

impl ForeignContext for Keccak256Context {
    fn get_statics(&self) -> Option<ForeignStatics> {
        Some(ForeignStatics {
            used_round: self.used_round,
            max_round: KeccakChip::max_rounds(zkwasm_k() as usize),
        })
    }
}

use specs::external_host_call_table::ExternalHostCallSignature;
pub fn register_keccak_foreign(env: &mut HostEnv) {
    let foreign_keccak_plugin = env
        .external_env
        .register_plugin("foreign_keccak", Box::new(Keccak256Context::default()));

    env.external_env.register_function(
        "keccak_new",
        Keccak256New as usize,
        ExternalHostCallSignature::Argument,
        foreign_keccak_plugin.clone(),
        Rc::new(
            |_obs: &Observer, context: &mut dyn ForeignContext, args: wasmi::RuntimeArgs| {
                let context = context.downcast_mut::<Keccak256Context>().unwrap();
                log::debug!("buf len is {}", context.buf.len());
                context.keccak_new(args.nth::<u64>(0) as usize);
                None
            },
        ),
    );

    env.external_env.register_function(
        "keccak_push",
        Keccak256Push as usize,
        ExternalHostCallSignature::Argument,
        foreign_keccak_plugin.clone(),
        Rc::new(
            |_obs: &Observer, context: &mut dyn ForeignContext, args: wasmi::RuntimeArgs| {
                let context = context.downcast_mut::<Keccak256Context>().unwrap();
                context.keccak_push(args.nth::<u64>(0) as u64);
                None
            },
        ),
    );

    env.external_env.register_function(
        "keccak_finalize",
        Keccak256Finalize as usize,
        ExternalHostCallSignature::Return,
        foreign_keccak_plugin.clone(),
        Rc::new(
            |_obs: &Observer, context: &mut dyn ForeignContext, _args: wasmi::RuntimeArgs| {
                let context = context.downcast_mut::<Keccak256Context>().unwrap();
                Some(wasmi::RuntimeValue::I64(context.keccak_finalize() as i64))
            },
        ),
    );
}
