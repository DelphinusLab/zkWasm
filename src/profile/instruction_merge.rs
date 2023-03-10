use std::collections::BTreeMap;

use log::debug;
use specs::{
    etable::EventTable,
    itable::{Opcode, OpcodeClass},
};

pub trait InstructionMergingProfile {
    fn estimate_mergeable_instruction(&self);
}

impl InstructionMergingProfile for EventTable {
    fn estimate_mergeable_instruction(&self) {
        let mut const_count = 0;
        let mut local_get = 0;
        let mut load_count = 0;
        let mut global_get_count = 0;

        let mut const_opt: BTreeMap<OpcodeClass, usize> = BTreeMap::new();
        let mut local_get_opt: BTreeMap<OpcodeClass, usize> = BTreeMap::new();

        self.entries()
            .iter()
            .zip(self.entries().iter().skip(1))
            .for_each(|(entry, next_entry)| {
                match (&entry.inst.opcode, &next_entry.inst.opcode) {
                    (Opcode::Const { .. }, Opcode::Bin { .. }) => {
                        const_count = const_count + 1;
                    }
                    (Opcode::LocalGet { .. }, Opcode::Bin { .. }) => {
                        local_get = local_get + 1;
                    }
                    (Opcode::Load { .. }, Opcode::Bin { .. }) => {
                        load_count = load_count + 1;
                    }
                    (Opcode::GlobalGet { .. }, Opcode::Bin { .. }) => {
                        global_get_count = global_get_count + 1;
                    }
                    _ => (),
                }

                if let Opcode::Const { .. } = &entry.inst.opcode {
                    match const_opt.get_mut(&(next_entry.inst.opcode.clone().into())) {
                        Some(counter) => *counter = *counter + 1,
                        None => {
                            const_opt.insert(next_entry.inst.opcode.clone().into(), 1);
                        }
                    }
                }

                if let Opcode::LocalGet { .. } = &entry.inst.opcode {
                    match local_get_opt.get_mut(&(next_entry.inst.opcode.clone().into())) {
                        Some(counter) => *counter = *counter + 1,
                        None => {
                            local_get_opt.insert(next_entry.inst.opcode.clone().into(), 1);
                        }
                    }
                }
            });

        debug!("mergeable instruction");
        debug!("local get: {}", local_get);
        debug!("const: {}", const_count);
        debug!("load: {}", load_count);
        debug!("global get: {}", global_get_count);

        debug!("const follow: {:?}", const_opt);
        debug!("local get follow: {:?}", local_get_opt);
    }
}
