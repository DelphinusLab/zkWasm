use std::cmp::Ordering;
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

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Commit {
    Unset,
    Set(usize),
}

impl Ord for Commit {
    fn cmp(&self, other: &Self) -> Ordering {
        match (self, other) {
            (Commit::Unset, Commit::Unset) => Ordering::Equal,
            (Commit::Unset, Commit::Set(_)) => Ordering::Greater,
            (Commit::Set(_), Commit::Unset) => Ordering::Less,
            (Commit::Set(a), Commit::Set(b)) => a.cmp(b),
        }
    }
}

impl PartialOrd for Commit {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Debug)]
struct Checkpoint {
    // transaction start index
    start: usize,
    commit: Commit,
}

#[derive(Debug)]
struct Checkpoints(Vec<Checkpoint>);

impl Checkpoints {
    fn merge(&mut self) {
        let mut checkpoints = std::mem::take(&mut self.0);
        checkpoints.sort_unstable();

        let mut merged = vec![checkpoints.remove(0)];
        checkpoints.into_iter().for_each(|checkpoint| {
            let last = merged.last_mut().unwrap();

            if Commit::Set(checkpoint.start) <= last.commit {
                last.commit = last.commit.max(checkpoint.commit);
            } else {
                merged.push(checkpoint)
            }
        });

        self.0 = merged;
    }

    fn abort(mut self, current: usize) -> usize {
        if self.0.is_empty() {
            return current;
        }

        self.merge();

        if self.0.last().unwrap().commit > Commit::Set(current) {
            self.0.last().unwrap().start
        } else {
            current
        }
    }
}

impl From<Vec<Checkpoint>> for Checkpoints {
    fn from(value: Vec<Checkpoint>) -> Self {
        Self(value)
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
    started: BTreeMap<TransactionId, Checkpoint>,
    committed: Vec<Checkpoint>,
    controller: Box<dyn FlushStrategy>,
    host_is_full: bool,

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
            started: BTreeMap::new(),
            committed: vec![],
            controller,
            host_is_full: false,
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
        if self.started.contains_key(&idx) {
            panic!("transaction id exists")
        }

        self.started.insert(
            idx,
            Checkpoint {
                start: self.now(),
                commit: Commit::Unset,
            },
        );
    }

    fn commit(&mut self, idx: TransactionId) {
        let mut transaction = self.started.remove(&idx).unwrap();
        transaction.commit = Commit::Set(self.now());
        self.committed.push(transaction);
    }

    fn abort(&mut self) {
        if self.len() == 0 {
            return;
        }

        let mut checkpoints = std::mem::take(&mut self.started)
            .into_values()
            .collect::<Vec<_>>();
        let mut committed = std::mem::take(&mut self.committed);
        checkpoints.append(&mut committed);
        let rollback = Checkpoints::from(checkpoints).abort(self.len());

        let mut logs = std::mem::take(&mut self.logs);

        let committed_logs = logs.drain(0..rollback);

        let slice = self.slice_builder.build(committed_logs.collect());
        self.slices.push(slice);

        // controller should be reset and we will replay the remaining logs
        self.host_is_full = false;
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
            StepInfo::ExternalHostCall { op, .. } => {
                if self.host_is_full {
                    self.abort();
                }

                self.controller.notify(Event::HostCall(op))
            }
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
                self.logs.push(log);
                self.commit(id);
            }
            Command::Abort => {
                self.insert(log);
                self.host_is_full = true;
            }
            Command::CommitAndAbort(id) => {
                self.logs.push(log);
                self.commit(id);
                self.host_is_full = true;
            }
        }
    }
}
