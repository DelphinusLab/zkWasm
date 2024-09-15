use serde::Deserialize;
use serde::Serialize;

use crate::etable::EventTable;
use crate::external_host_call_table::ExternalHostCallTable;
use crate::jtable::FrameTable;

pub mod memory;

#[derive(Serialize, Deserialize)]
pub struct Slice {
    pub etable: EventTable,
    pub frame_table: FrameTable,
    pub external_host_call_table: ExternalHostCallTable,
}

pub trait SliceBackend {
    fn push(&mut self, slice: Slice);
    fn pop(&mut self) -> Option<Slice>;
    fn first(&mut self) -> Option<&Slice>;
    fn len(&self) -> usize;
    fn is_empty(&self) -> bool;
    fn for_each<'a>(&'a self, f: Box<dyn Fn((usize, &Slice)) + 'a>);
}
