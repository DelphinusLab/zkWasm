pub mod sum;

use k256::elliptic_curve::point::DecompressPoint;
use k256::elliptic_curve::FieldBytesEncoding;
use k256::AffinePoint;

use delphinus_zkwasm::runtime::monitor::plugins::table::Command;
use k256::elliptic_curve::bigint::Limb;
use k256::elliptic_curve::bigint::U256;
use zkwasm_host_circuits::host::ForeignInst;
use zkwasm_host_circuits::proof::OpType;

const LIMBNB: usize = 4;

use crate::PluginFlushStrategy;

fn fetch_scalar(limbs: &[u64]) -> U256 {
    assert_eq!(limbs.len(), LIMBNB);
    let mut arr = [Limb::ZERO; U256::LIMBS];
    for (r, v) in arr.iter_mut().zip(limbs.iter()) {
        *r = Limb(*v)
    }
    U256::new(arr)
}

fn fetch_point(limbs: &Vec<u64>) -> AffinePoint {
    assert_eq!(limbs.len(), LIMBNB + 1);
    let x = fetch_scalar(&limbs[..LIMBNB]);
    let x_bytes = x.encode_field_bytes();
    let is_y_odd = limbs[LIMBNB] as u8;

    AffinePoint::decompress(&x_bytes, is_y_odd.into()).unwrap()
}

pub(crate) struct EcRecoverFlushStrategy {
    current: usize,
    group: usize,
    maximal_group: usize,
    new_ecrecover: bool,
}

impl EcRecoverFlushStrategy {
    pub(crate) fn new(_k: u32) -> Self {
        Self {
            current: 0,
            group: 0,
            // maximal_group: AltJubChip::<Fr>::max_rounds(k as usize),
            //todo
            maximal_group: 50,
            new_ecrecover: true,
        }
    }

    fn group_size() -> usize {
        // new + sig_r + sig_s + z + r_point +result point
        1 + 4 + 4 + 4 + 5 + 4
    }
}

impl PluginFlushStrategy for EcRecoverFlushStrategy {
    fn notify(&mut self, op: &ForeignInst, value: Option<u64>) -> Vec<Command> {
        let op_type = OpType::ECRECOVER as usize;

        self.current += 1;

        if *op as usize == ForeignInst::EcRecoverNew as usize {
            let value = value.unwrap();
            assert!(value == 0 || value == 1);

            self.new_ecrecover = value == 1;

            if self.new_ecrecover {
                return vec![Command::Finalize(op_type), Command::Start(op_type)];
            } else {
                return vec![Command::Start(op_type)];
            }
        }

        if self.current == EcRecoverFlushStrategy::group_size() {
            self.current = 0;
            self.group += 1;

            let mut commands = vec![Command::Commit(op_type, self.new_ecrecover)];

            if self.group >= self.maximal_group {
                commands.push(Command::Abort);
            }

            return commands;
        }

        vec![Command::Noop]
    }

    fn reset(&mut self) {
        self.current = 0;
        self.group = 0;
    }

    fn maximal_group(&self) -> Option<usize> {
        Some(self.maximal_group)
    }
}
