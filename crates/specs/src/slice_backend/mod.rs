use crate::etable::EventTable;
use crate::external_host_call_table::ExternalHostCallTable;
use crate::jtable::FrameTable;

pub mod memory;

pub struct Slice {
    pub etable: EventTable,
    pub frame_table: FrameTable,
    pub external_host_call_table: ExternalHostCallTable,
}

pub trait SliceBackend: Iterator<Item = Slice> {
    fn push(&mut self, slice: Slice);
    fn len(&self) -> usize;
    fn is_empty(&self) -> bool;
    // An renaming for_each to avoid conflict with Iterator trait and
    // to avoid consume self.
    fn for_each1<'a>(&'a self, f: Box<dyn Fn((usize, &Slice)) + 'a>);
}
