use std::borrow::Borrow;
use std::rc::Rc;

use specs::etable::EventTable;
use specs::etable::EventTableEntry;
use specs::step::StepInfo;
use specs::TableBackend;
use specs::TraceBackend;
use wasmi::DEFAULT_VALUE_STACK_LIMIT;

pub(super) struct ETable {
    //    pub(crate) eid: u32,
    slices: Vec<TableBackend<EventTable>>,
    //    entries: Vec<EventTableEntry>,
    capacity: u32,
    backend: Rc<TraceBackend>,
}

impl ETable {
    pub(crate) fn new(capacity: u32, backend: Rc<TraceBackend>) -> Self {
        Self {
            //            eid: 0,
            slices: Vec::default(),
            //            entries: Vec::with_capacity(capacity as usize),
            capacity,
            backend,
        }
    }

    pub(super) fn push_slice(&mut self, slice: EventTable) {
        let table = match self.backend.borrow() {
            TraceBackend::File {
                event_table_writer, ..
            } => TableBackend::Json(event_table_writer(self.slices.len(), &slice)),
            TraceBackend::Memory => TableBackend::Memory(slice),
        };

        self.slices.push(table);
    }

    pub(super) fn finalized(self) -> Vec<TableBackend<EventTable>> {
        self.slices
    }

    // pub(crate) fn flush(&mut self) {
    //     let empty = Vec::with_capacity(self.capacity as usize);
    //     let entries = std::mem::replace(&mut self.entries, empty);

    //     let event_table = match self.backend.as_ref() {
    //         TraceBackend::File {
    //             event_table_writer, ..
    //         } => TableBackend::Json(event_table_writer(
    //             self.slices.len(),
    //             &EventTable::new(entries),
    //         )),
    //         TraceBackend::Memory => TableBackend::Memory(EventTable::new(entries)),
    //     };

    //     self.slices.push(event_table);
    // }

    // pub(crate) fn push(
    //     &mut self,
    //     fid: u32,
    //     iid: u32,
    //     sp: u32,
    //     allocated_memory_pages: u32,
    //     last_jump_eid: u32,
    //     step_info: StepInfo,
    // ) {
    //     self.eid += 1;

    //     let sp = (DEFAULT_VALUE_STACK_LIMIT as u32)
    //         .checked_sub(sp)
    //         .unwrap()
    //         .checked_sub(1)
    //         .unwrap();

    //     let eentry = EventTableEntry {
    //         eid: self.eid,
    //         fid,
    //         iid,
    //         sp,
    //         allocated_memory_pages,
    //         last_jump_eid,
    //         step_info,
    //     };

    //     self.entries.push(eentry);
    // }

    // pub(super) fn entries(&self) -> &[EventTableEntry] {
    //     &self.entries
    // }

    // pub(crate) fn entries_mut(&mut self) -> &mut Vec<EventTableEntry> {
    //     &mut self.entries
    // }
}
