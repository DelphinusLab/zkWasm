use specs::etable::EventTableEntry;
use specs::slice_backend::SliceBackendBuilder;

use super::frame_table_builder::FrameTableBuilder;

pub(super) mod v1;
pub(super) mod v2;

pub type TransactionId = usize;

pub(super) trait TransactionSlicer<B: SliceBackendBuilder> {
    fn push_event(&mut self, event: EventTableEntry);
    fn frame_table_builder_get(&self) -> &FrameTableBuilder;
    fn frame_table_builder_get_mut(&mut self) -> &mut FrameTableBuilder;
    fn finalize(self) -> Vec<B::Output>;
}
