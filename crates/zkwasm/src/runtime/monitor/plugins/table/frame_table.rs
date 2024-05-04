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

impl Into<CalledFrameTableEntry> for &FrameTableEntry {
    fn into(self) -> CalledFrameTableEntry {
        assert!(!self.inherited);

        CalledFrameTableEntry(FrameTableEntryInternal {
            frame_id: self.frame_id,
            next_frame_id: self.next_frame_id,
            callee_fid: self.callee_fid,
            fid: self.fid,
            iid: self.iid,
            returned: self.returned,
        })
    }
}

impl Into<InheritedFrameTableEntry> for &FrameTableEntry {
    fn into(self) -> InheritedFrameTableEntry {
        assert!(self.inherited);

        InheritedFrameTableEntry {
            enable: true,
            internal: FrameTableEntryInternal {
                frame_id: self.frame_id,
                next_frame_id: self.next_frame_id,
                callee_fid: self.callee_fid,
                fid: self.fid,
                iid: self.iid,
                returned: self.returned,
            },
        }
    }
}

pub(super) struct FrameTable {
    initial_frame_entries: Vec<FrameTableEntry>,
    slices: Vec<TableBackend<specs::jtable::FrameTable>>,
    current: Vec<FrameTableEntry>,
    backend: Rc<TraceBackend>,
}

impl FrameTable {
    pub(super) fn new(backend: Rc<TraceBackend>) -> Self {
        Self {
            initial_frame_entries: Vec::new(),
            slices: Vec::new(),
            current: Vec::new(),
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
        self.current.push(FrameTableEntry {
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

        self.current.push(entry.clone());
        self.initial_frame_entries.push(entry);
    }

    // Prepare for the next slice. This will remove all the entries that are returned
    pub(super) fn flush(&mut self) {
        let frame_table = {
            let frame_table = specs::jtable::FrameTable {
                inherited: Arc::new(
                    self.current
                        .iter()
                        .filter(|entry| entry.inherited)
                        .map(Into::into)
                        .collect::<Vec<InheritedFrameTableEntry>>()
                        .into(),
                ),
                called: CalledFrameTable::new(
                    self.current
                        .iter()
                        .filter(|entry| !entry.inherited)
                        .map(Into::into)
                        .collect(),
                ),
            };

            match self.backend.as_ref() {
                TraceBackend::Memory => TableBackend::Memory(frame_table),
                TraceBackend::File {
                    frame_table_writer, ..
                } => TableBackend::Json(frame_table_writer(self.slices.len(), &frame_table)),
            }
        };

        self.slices.push(frame_table);

        self.current.retain(|entry| !entry.returned);
        for entry in self.current.iter_mut() {
            entry.inherited = true;
        }
    }

    pub(super) fn pop(&mut self) {
        // get the last frame entry which returned is false.
        // TODO: add a cursor instead of finding
        let last = self.current.iter_mut().rev().find(|entry| !entry.returned);

        last.unwrap().returned = true;
    }

    pub(super) fn finalized(mut self) -> Vec<TableBackend<specs::jtable::FrameTable>> {
        self.flush();

        assert!(self.current.is_empty(), "all frames should be returned");

        self.slices
    }

    pub(super) fn build_initial_frame_table(&self) -> InheritedFrameTable {
        self.initial_frame_entries
            .iter()
            .map(|entry| InheritedFrameTableEntry {
                enable: true,
                internal: FrameTableEntryInternal {
                    frame_id: entry.frame_id,
                    next_frame_id: entry.next_frame_id,
                    callee_fid: entry.callee_fid,
                    fid: entry.fid,
                    iid: entry.iid,
                    returned: false,
                },
            })
            .collect::<Vec<_>>()
            .try_into()
            .unwrap()
    }
}
