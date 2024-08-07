use std::collections::BTreeMap;
use std::usize;

use specs::etable::EventTable;
use specs::etable::EventTableEntry;
use specs::external_host_call_table::ExternalHostCallTable;
use specs::jtable::FrameTable;
use specs::step::StepInfo;
use specs::TableBackend;
use specs::TraceBackend;

use crate::runtime::monitor::plugins::table::Event;

use super::slice_builder::SliceBuilder;
use super::Command;
use super::FlushStrategy;
use super::Slice;

pub(crate) type TransactionId = usize;

#[derive(PartialEq, PartialOrd, Eq, Ord)]
struct Checkpoint {
    // transaction start index
    start: usize,
    commit: Option<usize>,
}

struct Checkpoints(Vec<Checkpoint>);

impl Checkpoints {
    fn poison_uncommitted(&mut self) {
        for checkpoint in &mut self.0 {
            if checkpoint.commit.is_none() {
                checkpoint.commit = Some(usize::MAX);
            }
        }
    }

    fn merge(&mut self) {
        let mut checkpoints = std::mem::take(&mut self.0);
        checkpoints.sort_unstable();

        let mut merged = vec![checkpoints.remove(0)];
        checkpoints.into_iter().for_each(|checkpoint| {
            let last = merged.last_mut().unwrap();

            if checkpoint.start <= last.commit.unwrap() {
                last.commit = Some(last.commit.unwrap().max(checkpoint.commit.unwrap()));
            } else {
                merged.push(checkpoint)
            }
        });

        self.0 = merged;
    }

    fn abort(mut self) -> Option<usize> {
        if self.0.is_empty() {
            return None;
        }

        self.poison_uncommitted();
        self.merge();

        Some(self.0.last().unwrap().start)
    }
}

impl<T> From<BTreeMap<T, Checkpoint>> for Checkpoints {
    fn from(value: BTreeMap<T, Checkpoint>) -> Self {
        Self(value.into_values().into_iter().collect())
    }
}

pub(super) struct Slices {
    backend: TraceBackend,
    pub(super) etable: Vec<TableBackend<EventTable>>,
    pub(super) frame_table: Vec<TableBackend<FrameTable>>,
    pub(super) external_host_call_table: Vec<ExternalHostCallTable>,
}

impl Slices {
    fn new(backend: TraceBackend) -> Self {
        Self {
            backend,

            etable: Vec::new(),
            frame_table: Vec::new(),
            external_host_call_table: Vec::new(),
        }
    }

    fn push(&mut self, slice: Slice) {
        let (etable, frame_table) = match &self.backend {
            TraceBackend::File {
                event_table_writer,
                frame_table_writer,
            } => {
                let etable =
                    TableBackend::Json(event_table_writer(self.etable.len(), &slice.etable));
                let frame_table = TableBackend::Json(frame_table_writer(
                    self.frame_table.len(),
                    &slice.frame_table,
                ));

                (etable, frame_table)
            }
            TraceBackend::Memory => {
                let etable = TableBackend::Memory(slice.etable);
                let frame_table = TableBackend::Memory(slice.frame_table);

                (etable, frame_table)
            }
        };

        self.etable.push(etable);
        self.frame_table.push(frame_table);
        self.external_host_call_table
            .push(slice.external_host_call_table);
    }
}

pub(super) struct HostTransaction {
    slices: Slices,
    capacity: u32,

    logs: Vec<EventTableEntry>,
    committed: BTreeMap<TransactionId, Checkpoint>,
    controller: Box<dyn FlushStrategy>,

    pub(crate) slice_builder: SliceBuilder,
}

impl HostTransaction {
    pub(super) fn new(
        backend: TraceBackend,
        capacity: u32,
        controller: Box<dyn FlushStrategy>,
    ) -> Self {
        Self {
            slices: Slices::new(backend),
            slice_builder: SliceBuilder::new(),
            capacity,

            logs: Vec::new(),
            committed: BTreeMap::new(),
            controller,
        }
    }

    fn now(&self) -> usize {
        self.logs.len()
    }

    pub(super) fn len(&self) -> usize {
        self.logs.len()
    }

    // begin the transaction
    fn start(&mut self, idx: TransactionId) {
        if self.committed.contains_key(&idx) {
            panic!("transaction id exists")
        }

        self.committed.insert(
            idx,
            Checkpoint {
                start: self.now(),
                commit: None,
            },
        );
    }

    fn commit(&mut self, idx: TransactionId) {
        self.committed.get_mut(&idx).unwrap().commit = Some(self.now())
    }

    fn abort(&mut self) {
        if self.len() == 0 {
            return;
        }

        let checkpoints = std::mem::take(&mut self.committed);
        let rollback = Checkpoints::from(checkpoints).abort().unwrap_or(self.len());

        let mut logs = std::mem::take(&mut self.logs);

        let committed_logs = logs.drain(0..rollback);

        let slice = self.slice_builder.build(committed_logs.collect());
        self.slices.push(slice);

        let command = self.controller.notify(Event::Reset);
        assert!(command == Command::Noop);

        self.replay(logs);
    }

    pub(super) fn finalized(mut self) -> Slices {
        self.abort();

        self.slices
    }
}

impl HostTransaction {
    fn replay(&mut self, logs: Vec<EventTableEntry>) {
        for log in logs {
            self.insert(log);
        }
    }

    pub(crate) fn insert(&mut self, log: EventTableEntry) {
        if self.logs.len() == self.capacity as usize {
            self.abort();
        }

        let command = match log.step_info {
            StepInfo::ExternalHostCall { op, .. } => self.controller.notify(Event::HostCall(op)),
            _ => Command::Noop,
        };

        match command {
            Command::Noop => {
                self.logs.push(log);
            }
            Command::Start(id) => {
                self.start(id);
                self.logs.push(log);
            }
            Command::Commit(id) => {
                self.commit(id);
                self.logs.push(log);
            }
            Command::Abort => {
                self.abort();
                self.insert(log);
            }
        }
    }
}
