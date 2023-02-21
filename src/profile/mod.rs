use std::{collections::BTreeMap, fmt::Debug};

use specs::{
    etable::EventTable,
    itable::OpcodeClass,
    mtable::{AccessType, MemoryTableEntry},
};

use crate::runtime::memory_event_of_step;

pub trait Profile {
    fn profile_instruction(&self);
}

impl Profile for EventTable {
    fn profile_instruction(&self) {
        struct Counter {
            counter: usize,
            mentries: Vec<MemoryTableEntry>,
        }

        let mut map = BTreeMap::<OpcodeClass, Counter>::new();
        for entry in self.entries() {
            let mut mentries = memory_event_of_step(entry, &mut 1);

            if let Some(counter) = map.get_mut(&entry.inst.opcode.clone().into()) {
                counter.counter += 1;
                counter.mentries.append(&mut mentries);
            } else {
                map.insert(
                    entry.inst.opcode.clone().into(),
                    Counter {
                        counter: 1,
                        mentries,
                    },
                );
            }
        }

        let total_mentries: usize = {
            let a = map
                .values()
                .map(|counter| counter.mentries.len())
                .collect::<Vec<_>>();
            a.iter().sum()
        };

        println!("etable entries: {}", self.entries().len());
        println!("mtable entries: {}", total_mentries);

        struct Summary {
            counter: usize,
            mentries: usize,
            total_mentries: usize,
            write_counter: usize,
            read_counter: usize,
        }

        impl Debug for Summary {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(
                    f,
                    "{{ counter: {}, mentries: {}({:.2}%), write: {}({:.2}%), read: {}({:.2}%) }}",
                    self.counter,
                    self.mentries,
                    self.mentries as f64 / self.total_mentries as f64 * 100f64,
                    self.write_counter,
                    self.write_counter as f64 / self.total_mentries as f64 * 100f64,
                    self.read_counter,
                    self.read_counter as f64 / self.total_mentries as f64 * 100f64,
                )
            }
        }

        let summary = map
            .into_iter()
            .map(|(inst, counter)| {
                (
                    inst,
                    Summary {
                        counter: counter.counter,
                        mentries: counter.mentries.len(),
                        total_mentries,
                        write_counter: counter
                            .mentries
                            .iter()
                            .filter(|entry| entry.atype == AccessType::Write)
                            .collect::<Vec<_>>()
                            .len(),
                        read_counter: counter
                            .mentries
                            .iter()
                            .filter(|entry| entry.atype == AccessType::Read)
                            .collect::<Vec<_>>()
                            .len(),
                    },
                )
            })
            .collect::<BTreeMap<_, _>>();

        println!("{:?}", summary);
    }
}
