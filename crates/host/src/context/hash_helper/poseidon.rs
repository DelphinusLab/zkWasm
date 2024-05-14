use ff::PrimeField;
use halo2_proofs::pairing::bn256::Fr;
use poseidon::Poseidon;
pub use zkwasm_host_circuits::host::poseidon::POSEIDON_HASHER;
use zkwasm_host_circuits::host::Reduce;
use zkwasm_host_circuits::host::ReduceRule;

/// Foreign functions that supports the following C code library
///
/// void poseidon(uint64_t* data, uint32_t size, uint64_t* r)
/// {
///     int i;
///     poseidon_new(size);
///     for(i=0; i<size; i=++) {
///         uint64_t* a = data[i];
///         poseidon_push(data[i]);
///     }
///     r[0] = poseidon_finalize();
///     r[1] = poseidon_finalize();
///     r[2] = poseidon_finalize();
///     r[3] = poseidon_finalize();
///     wasm_dbg(r[0]);
///     wasm_dbg(r[1]);
///     wasm_dbg(r[2]);
///     wasm_dbg(r[3]);
/// }

pub struct Generator {
    pub cursor: usize,
    pub values: Vec<u64>,
}

impl Generator {
    pub fn gen(&mut self) -> u64 {
        let r = self.values[self.cursor];
        self.cursor += 1;
        if self.cursor == 4 {
            self.cursor = 0;
        }
        r
    }
}

pub fn new_reduce(rules: Vec<ReduceRule<Fr>>) -> Reduce<Fr> {
    Reduce { cursor: 0, rules }
}

pub struct PoseidonContext {
    pub k: u32,
    pub hasher: Option<Poseidon<Fr, 9, 8>>,
    pub generator: Generator,
    pub buf: Vec<Fr>,
    pub fieldreducer: Reduce<Fr>,
    pub used_round: usize,
}

impl PoseidonContext {
    pub fn default(k: u32) -> Self {
        PoseidonContext {
            k,
            hasher: None,
            fieldreducer: new_reduce(vec![ReduceRule::Field(Fr::zero(), 64)]),
            buf: vec![],
            generator: Generator {
                cursor: 0,
                values: vec![],
            },
            used_round: 0,
        }
    }

    pub fn poseidon_new(&mut self, new: usize) {
        self.buf = vec![];
        if new != 0 {
            self.hasher = Some(POSEIDON_HASHER.clone());
            self.used_round += 1;
        }
    }

    pub fn poseidon_push(&mut self, v: u64) {
        self.fieldreducer.reduce(v);
        if self.fieldreducer.cursor == 0 {
            self.buf
                .push(self.fieldreducer.rules[0].field_value().unwrap())
        }
    }

    pub fn poseidon_finalize(&mut self) -> u64 {
        assert!(self.buf.len() == 8);
        if self.generator.cursor == 0 {
            self.hasher.as_mut().map(|s| {
                log::debug!("perform hash with {:?}", self.buf);
                let r = s.update_exact(&self.buf.clone().try_into().unwrap());
                let dwords: Vec<u8> = r.to_repr().to_vec();
                self.generator.values = dwords
                    .chunks(8)
                    .map(|x| u64::from_le_bytes(x.to_vec().try_into().unwrap()))
                    .collect::<Vec<u64>>();
            });
        }
        self.generator.gen()
    }
}
