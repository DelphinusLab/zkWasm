use std::collections::BTreeMap;
use std::usize;

use specs::etable::EventTable;
use specs::etable::EventTableEntry;
use specs::external_host_call_table::ExternalHostCallTable;
use specs::jtable::FrameTable;
use specs::step::StepInfo;

use super::Command;
use super::Error;
use super::Event;
use super::FlushStrategy;
use super::MonitorError;
use super::Slice;

pub(crate) type TransactionId = usize;

impl From<&[EventTableEntry]> for Slice {
    fn from(_value: &[EventTableEntry]) -> Self {
        todo!()
    }
}

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

    fn abort(mut self) -> usize {
        self.poison_uncommitted();
        self.merge();

        self.0.last().unwrap().start
    }
}

impl<T> From<BTreeMap<T, Checkpoint>> for Checkpoints {
    fn from(value: BTreeMap<T, Checkpoint>) -> Self {
        Self(value.into_values().into_iter().collect())
    }
}

pub(super) struct HostTransaction {
    logs: Vec<EventTableEntry>,
    committed: BTreeMap<TransactionId, Checkpoint>,
    controller: Box<dyn FlushStrategy>,
}

impl HostTransaction {
    pub(super) fn new(controller: Box<dyn FlushStrategy>) -> Self {
        Self {
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

    pub(super) fn abort(&mut self) -> Result<Slice, MonitorError> {
        let checkpoints = std::mem::take(&mut self.committed);
        let rollback = Checkpoints::from(checkpoints).abort();

        let mut logs = std::mem::take(&mut self.logs);

        let committed_logs = logs.drain(0..rollback);
        let slice = self.create_slice(committed_logs.as_slice());
        drop(committed_logs);

        self.controller.notify(Event::Reset)?;
        self.replay(logs);

        Ok(slice)
    }

    pub(super) fn assert_empty(&self) -> Result<(), MonitorError> {
        if self.logs.is_empty() {
            Ok(())
        } else {
            Err(MonitorError::TerminateWithUncommitted)
        }
    }
}

impl HostTransaction {
    fn replay(&mut self, logs: Vec<EventTableEntry>) -> Result<(), MonitorError> {
        for log in logs {
            self.append(log)?;
        }

        Ok(())
    }

    pub(crate) fn append(&mut self, log: EventTableEntry) -> Result<(), MonitorError> {
        let command = match log.step_info {
            StepInfo::ExternalHostCall { op, .. } => self.controller.notify(Event::HostCall(op))?,
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
                self.abort()?;
                self.append(log)?;
            }
        }

        Ok(())
    }

    fn create_slice(&self, entries: &[EventTableEntry]) -> Slice {
        Slice::from(entries)
    }

    pub(super) fn nofity(&mut self, event: Event) -> Result<Command, MonitorError> {
        self.controller.notify(event)
    }
}
