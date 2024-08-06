use std::borrow::Borrow;
use std::rc::Rc;
use std::sync::Arc;

use specs::jtable::CalledFrameTable;
use specs::jtable::CalledFrameTableEntry;
use specs::jtable::FrameTableEntryInternal;
use specs::jtable::InheritedFrameTable;
use specs::jtable::InheritedFrameTableEntry;
use specs::TableBackend;
use specs::TraceBackend;

#[derive(Clone)]
struct FrameTableEntry {
    frame_id: u32,
    next_frame_id: u32,
    callee_fid: u32,
    fid: u32,
    iid: u32,
    inherited: bool,
    returned: bool,
}

impl From<&FrameTableEntry> for CalledFrameTableEntry {
    fn from(entry: &FrameTableEntry) -> CalledFrameTableEntry {
        assert!(!entry.inherited);

        CalledFrameTableEntry(FrameTableEntryInternal {
            frame_id: entry.frame_id,
            next_frame_id: entry.next_frame_id,
            callee_fid: entry.callee_fid,
            fid: entry.fid,
            iid: entry.iid,
            returned: entry.returned,
        })
    }
}

impl From<&FrameTableEntry> for InheritedFrameTableEntry {
    fn from(entry: &FrameTableEntry) -> InheritedFrameTableEntry {
        assert!(entry.inherited);

        InheritedFrameTableEntry(Some(FrameTableEntryInternal {
            frame_id: entry.frame_id,
            next_frame_id: entry.next_frame_id,
            callee_fid: entry.callee_fid,
            fid: entry.fid,
            iid: entry.iid,
            returned: entry.returned,
        }))
    }
}

pub(super) struct FrameTable {
    initial_frame_entries: Vec<FrameTableEntry>,
    slices: Vec<TableBackend<specs::jtable::FrameTable>>,

    current_unreturned: Vec<FrameTableEntry>,
    current_returned: Vec<FrameTableEntry>,

    backend: Rc<TraceBackend>,
}

impl FrameTable {
    pub(super) fn new(backend: Rc<TraceBackend>) -> Self {
        Self {
            initial_frame_entries: Vec::new(),
            slices: Vec::new(),

            current_unreturned: Vec::new(),
            current_returned: Vec::new(),

            backend,
        }
    }

    pub(super) fn push(
        &mut self,
        frame_id: u32,
        next_frame_id: u32,
        callee_fid: u32,
        fid: u32,
        iid: u32,
    ) {
        self.current_unreturned.push(FrameTableEntry {
            frame_id,
            next_frame_id,
            callee_fid,
            fid,
            iid,
            inherited: false,
            returned: false,
        });
    }

    pub(super) fn push_static_entry(
        &mut self,
        frame_id: u32,
        next_frame_id: u32,
        callee_fid: u32,
        fid: u32,
        iid: u32,
    ) {
        let entry = FrameTableEntry {
            frame_id,
            next_frame_id,
            callee_fid,
            fid,
            iid,
            inherited: true,
            returned: false,
        };

        self.current_unreturned.push(entry.clone());
        self.initial_frame_entries.push(entry);
    }

    pub(super) fn push_slice(&mut self, frame_table: specs::jtable::FrameTable) {
        let slice = match self.backend.as_ref() {
            TraceBackend::Memory => TableBackend::Memory(frame_table),
            TraceBackend::File {
                frame_table_writer, ..
            } => TableBackend::Json(frame_table_writer(self.slices.len(), &frame_table)),
        };

        self.slices.push(slice);
    }

    // Prepare for the next slice. This will remove all the entries that are returned
    pub(super) fn flush(&mut self) {
        let frame_table = {
            let frame_table = {
                let inherited = self
                    .current_returned
                    .iter()
                    .chain(self.current_unreturned.iter())
                    .filter(|entry| entry.inherited)
                    .map(Into::into)
                    .collect::<Vec<InheritedFrameTableEntry>>();

                let called = self
                    .current_returned
                    .iter()
                    .chain(self.current_unreturned.iter())
                    .filter(|entry| !entry.inherited)
                    .map(Into::into)
                    .collect::<Vec<CalledFrameTableEntry>>();

                specs::jtable::FrameTable {
                    inherited: Arc::new(inherited.into()),
                    called: CalledFrameTable::new(called),
                }
            };

            match self.backend.as_ref() {
                TraceBackend::Memory => TableBackend::Memory(frame_table),
                TraceBackend::File {
                    frame_table_writer, ..
                } => TableBackend::Json(frame_table_writer(self.slices.len(), &frame_table)),
            }
        };

        self.slices.push(frame_table);

        self.current_returned.clear();
        for entry in self.current_unreturned.iter_mut() {
            entry.inherited = true;
        }
    }

    pub(super) fn pop(&mut self) {
        let mut entry = self.current_unreturned.pop().unwrap();
        entry.returned = true;
        self.current_returned.push(entry);
    }

    pub(super) fn finalized(mut self) -> Vec<TableBackend<specs::jtable::FrameTable>> {
        // self.flush();

        // assert!(
        //     self.current_unreturned.is_empty(),
        //     "all frames should be returned"
        // );

        self.slices
    }

    pub(super) fn build_initial_frame_table(&self) -> InheritedFrameTable {
        self.initial_frame_entries
            .iter()
            .map(|entry| {
                InheritedFrameTableEntry(Some(FrameTableEntryInternal {
                    frame_id: entry.frame_id,
                    next_frame_id: entry.next_frame_id,
                    callee_fid: entry.callee_fid,
                    fid: entry.fid,
                    iid: entry.iid,
                    returned: false,
                }))
            })
            .collect::<Vec<_>>()
            .try_into()
            .unwrap()
    }
}
