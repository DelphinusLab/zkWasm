use super::{ExternalHostCallEntry, ExternalHostCallTable};
use crate::{etable::EventTable, step::StepInfo};

impl EventTable {
    pub fn filter_external_host_call_table(&self) -> ExternalHostCallTable {
        let entries = self
            .entries()
            .iter()
            .filter_map(|entry| {
                if let StepInfo::ExternalHostCall { op, value, sig, .. } = &entry.step_info {
                    Some(ExternalHostCallEntry {
                        op: *op,
                        value: value.unwrap(),
                        sig: *sig,
                    })
                } else {
                    None
                }
            })
            .collect();

        ExternalHostCallTable(entries)
    }
}
