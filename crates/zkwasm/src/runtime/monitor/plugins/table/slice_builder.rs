use specs::etable::EventTable;
use specs::etable::EventTableEntry;
use specs::external_host_call_table::ExternalHostCallEntry;
use specs::external_host_call_table::ExternalHostCallTable;
use specs::slice_backend::Slice;

use super::frame_table_builder::FrameTableBuilder;

pub struct SliceBuilder {
    pub(super) frame_table_builder: FrameTableBuilder,
}

impl SliceBuilder {
    pub(super) fn new() -> Self {
        SliceBuilder {
            frame_table_builder: FrameTableBuilder::new(),
        }
    }

    pub(super) fn build(&mut self, logs: Vec<EventTableEntry>) -> Slice {
        let external_host_call_table = ExternalHostCallTable::new(
            logs.iter()
                .filter_map(|entry| ExternalHostCallEntry::try_from(&entry.step_info).ok())
                .collect(),
        );

        let frame_table = self.frame_table_builder.build(&logs[..]);

        Slice {
            etable: EventTable::new(logs),
            frame_table,
            external_host_call_table,
        }
    }
}
