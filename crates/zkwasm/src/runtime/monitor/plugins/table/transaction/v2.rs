use std::cell::RefCell;
use std::cmp::Ordering;
use std::collections::HashMap;
use std::collections::HashSet;
use std::collections::LinkedList;
use std::rc::Rc;

use log::warn;
use specs::etable::EventTableEntry;
use specs::mtable::AccessType;
use specs::slice_backend::SliceBackendBuilder;
use specs::step::StepInfo;

use crate::runtime::memory_event_of_step;
use crate::runtime::monitor::plugins::table::frame_table_builder::FrameTableBuilder;
use crate::runtime::monitor::plugins::table::slice_builder::SliceBuilder;
use crate::runtime::monitor::plugins::table::Command;
use crate::runtime::monitor::plugins::table::Event;
use crate::runtime::monitor::plugins::table::FlushStrategy;

use super::TransactionId;
use super::TransactionSlicer;

const MAX_SLICES_IN_MEMORY: usize = 3;
const TIMER_DELAY: usize = 2;

#[derive(Clone)]
struct Range {
    start: usize,
    // Inclusive bound
    end: Option<usize>,
}

impl Range {
    fn contains(&self, offset: usize) -> bool {
        if let Some(end) = self.end {
            self.start <= offset && offset <= end
        } else {
            self.start <= offset
        }
    }

    fn before(&self, offset: usize) -> bool {
        self.end.map(|end| end < offset).unwrap_or(false)
    }

    fn after(&self, offset: usize) -> bool {
        self.start > offset
    }

    fn contains_or_after(&self, offset: usize) -> bool {
        self.contains(offset) || self.after(offset)
    }
}

#[derive(Clone)]
struct Checkpoint {
    range: Range,
    weak_dependencies: HashMap<TransactionId, usize>,
    // how many transactions it includes
    transactions_group_number: HashMap<TransactionId, usize>,
}

impl Checkpoint {
    fn has_dependencies(&self) -> bool {
        self.weak_dependencies
            .iter()
            .any(|(_tx, count)| *count != 0)
    }

    // return value:
    // Ordering::Greater: at least one of transaction is overflow
    // Ordering::Equal: all transactions are full
    // Ordering::Less : no transaction overflow and at least one of transacion is not full
    fn transactions_group_number_ordering(
        &self,
        applied_transactions_group_number: &HashMap<TransactionId, usize>,
        flush_strategy_controller: &dyn FlushStrategy,
    ) -> Ordering {
        let mut ordering = Ordering::Equal;

        for (tx, group_number) in &self.transactions_group_number {
            if let Some(limit) = flush_strategy_controller.maximal_group(*tx) {
                let applied = applied_transactions_group_number
                    .get(tx)
                    .cloned()
                    .unwrap_or_default();

                let number = *group_number - applied;
                match number.cmp(&limit) {
                    Ordering::Less => ordering = Ordering::Less,
                    Ordering::Equal => {}
                    Ordering::Greater => return Ordering::Greater,
                }
            }
        }

        ordering
    }
}

struct WeakCommittedTransaction {
    first_tx_start: usize,
    last: usize,
    count: usize,
}

struct Checkpoints {
    slice_capacity: usize,

    // the group number of the transaction which applied in slice
    applied_transactions_group_number: HashMap<TransactionId, usize>,
    // how many committed transactions exist now
    total_transactions_group_number: HashMap<TransactionId, usize>,

    // uncommitted transaction and its start offset
    transactions: HashMap<TransactionId, usize>,
    // committed weak transactions
    weak_committed: HashMap<TransactionId, WeakCommittedTransaction>,

    checkpoints: Vec<Checkpoint>,
}

impl Checkpoints {
    fn new(slice_capacity: usize) -> Self {
        Self {
            slice_capacity,
            applied_transactions_group_number: HashMap::default(),
            total_transactions_group_number: HashMap::default(),
            transactions: HashMap::default(),
            weak_committed: HashMap::default(),
            checkpoints: vec![Checkpoint {
                range: Range {
                    start: 0,
                    end: None,
                },
                weak_dependencies: HashMap::default(),
                transactions_group_number: HashMap::default(),
            }],
        }
    }

    fn start(&mut self, tx: TransactionId, offset: usize) {
        // end the lastest checkpoint
        if self.transactions.is_empty() {
            if let Some(checkpoint) = self.checkpoints.last_mut() {
                checkpoint.range.end = Some(offset);
            }
        }

        let old = self.transactions.insert(tx, offset);
        assert!(old.is_none(), "recursive transaction is not supported yet");
    }

    fn commit(&mut self, tx: TransactionId, offset: usize) -> usize {
        let start = self
            .transactions
            .remove(&tx)
            .unwrap_or_else(|| panic!("commit a not existing transaction {}", tx));

        let committed_tx = self.total_transactions_group_number.entry(tx).or_default();
        *committed_tx += 1;

        self.weak_committed
            .entry(tx)
            .and_modify(|tx| {
                tx.count += 1;
                tx.last = offset;
            })
            .or_insert(WeakCommittedTransaction {
                first_tx_start: start,
                last: offset,
                count: 1,
            });

        if self.transactions.is_empty() {
            self.insert_checkpoint(offset);
        }

        start
    }

    // finalize all transactions of 'tx' to now, it is active called by host
    fn finalize(&mut self, tx: TransactionId) {
        let desc = self.weak_committed.remove(&tx);

        if let Some(desc) = desc {
            if desc.last - desc.first_tx_start > self.slice_capacity {
                panic!(
                    "Transactions (transaction id: {}, count: {}) cannot be placed in \
                a slice because the first transaction(start at {}) is more than \
                the slice size away from the last transaction(commit at {})",
                    tx, desc.count, desc.first_tx_start, desc.last
                );
            }

            self.active_release_weak_dependencies_for_checkpoint_from(tx, desc.last);
        }
    }

    // called due to termination
    fn finalize_all(&mut self) {
        let weak_committed = self.weak_committed.keys().cloned().collect::<Vec<_>>();

        for tx in weak_committed {
            self.finalize(tx);
        }
    }

    // Only finalized checkpoint can be applied
    // release 'n' weak transactions after 'from''
    // fn force_commit_weak_n(&mut self, tx: TransactionId, from: usize, n: usize) {
    //     let have_started_new =
    //         self.passive_release_weak_dependencies_for_checkpoint_from(tx, from, n);

    //     if !have_started_new {
    //         let desc = self.weak_committed.get_mut(&tx).unwrap();
    //         desc.count = desc.count.checked_sub(n).unwrap();

    //         if desc.count == 0 {
    //             self.weak_committed.remove(&tx);
    //         }
    //     }
    // }

    fn insert_checkpoint(&mut self, offset: usize) {
        let weak_dependencies = self
            .weak_committed
            .iter()
            .filter_map(|(tx, desc)| {
                if desc.count > 0 {
                    Some((*tx, desc.count))
                } else {
                    None
                }
            })
            .collect();

        self.checkpoints.push(Checkpoint {
            range: Range {
                start: offset,
                end: None,
            },
            weak_dependencies,
            transactions_group_number: self.total_transactions_group_number.clone(),
        });
    }

    // Only finalized checkpoint can be applied
    // return value indicates whether a new id transaction have started
    // fn passive_release_weak_dependencies_for_checkpoint_from(
    //     &mut self,
    //     tx: TransactionId,
    //     from: usize,
    //     n: usize,
    // ) -> bool {
    //     assert!(n > 0);

    //     for checkpoint in &mut self.checkpoints[from..] {
    //         let desc = checkpoint.weak_dependencies.get_mut(&tx);

    //         match desc {
    //             Some(count) if *count >= n => {
    //                 *count -= n;
    //             }
    //             // already finalized
    //             Some(count) => {
    //                 assert_eq!(*count, 0);
    //                 return true;
    //             }
    //             // already finalized
    //             None => return true,
    //         }
    //     }

    //     false
    // }

    // return value indicates whether a new id transaction have started
    fn active_release_weak_dependencies_for_checkpoint_from(
        &mut self,
        tx: TransactionId,
        from: usize,
    ) {
        for checkpoint in self.checkpoints.iter_mut().rev() {
            if checkpoint.range.contains_or_after(from) {
                checkpoint.weak_dependencies.remove(&tx);
            } else {
                break;
            }
        }
    }

    fn apply_checkpoint(&mut self, index: usize, upper_bound: usize) -> usize {
        let checkpoint = self.checkpoints[index].clone();

        if checkpoint.has_dependencies() {
            unreachable!("only firnalized checkpoint can be applied due to filter function");

            // warn!(
            //     "Create a slice with weak transactions: {:?}, this may fail host circuits",
            //     checkpoint.weak_dependencies
            // );

            // for (tx, n) in &checkpoint.weak_dependencies {
            //     self.force_commit_weak_n(*tx, index, *n);
            // }
        }

        for (tx, n) in checkpoint.transactions_group_number {
            self.applied_transactions_group_number
                .entry(tx)
                .and_modify(|count| *count = n)
                .or_insert(n);
        }

        let checkpoint = self.checkpoints[index].clone();

        if checkpoint.range.before(upper_bound) {
            self.checkpoints.drain(0..=index);

            checkpoint.range.end.unwrap()
        } else {
            let position = upper_bound;

            // if split
            if Some(position) != checkpoint.range.end {
                let checkpoint = self.checkpoints.get_mut(index).unwrap();
                checkpoint.range.start = position;
                self.checkpoints.drain(0..index);
            } else {
                self.checkpoints.drain(0..=index);
            }

            position
        }
    }

    fn find(
        &self,
        upper_bound: usize,
        filter: impl Fn(&Checkpoint) -> bool,
        flush_strategy_controller: &dyn FlushStrategy,
    ) -> Option<usize> {
        let last = self.checkpoints.binary_search_by(|checkpoint| {
            let group_number_ordering = checkpoint.transactions_group_number_ordering(
                &self.applied_transactions_group_number,
                flush_strategy_controller,
            );

            if checkpoint.range.after(upper_bound) || group_number_ordering.is_gt() {
                return Ordering::Greater;
            }

            if checkpoint.range.contains(upper_bound) && group_number_ordering.is_eq() {
                return Ordering::Equal;
            }

            Ordering::Less
        });
        let last = match last {
            Ok(index) => index,
            Err(index) => {
                assert!(index > 0);
                index - 1
            }
        };

        self.checkpoints[..=last]
            .iter()
            .enumerate()
            .rev()
            .find(|(_, checkpoint)| filter(checkpoint))
            .map(|(index, _)| index)
    }

    fn checkpoint(
        &mut self,
        upper_bound: usize,
        flush_strategy_controller: &dyn FlushStrategy,
    ) -> usize {
        // prefer checkpoint without weak dependencies
        let index = self.find(
            upper_bound,
            |checkpoint| !checkpoint.has_dependencies(),
            flush_strategy_controller,
        );

        self.apply_checkpoint(
            index.expect("cannot find a checkpoint to meet host circuit"),
            upper_bound,
        )
    }
}

struct Timer {
    tx: TransactionId,
    deadline: usize,
    // since Rust LinkedList doesn't support remove by node
    disabled: bool,
}

pub struct HostTransaction<B: SliceBackendBuilder> {
    event_table_capacity: usize,
    memory_table_capacity: usize,
    last_committed_event_cursor: usize,
    events: Vec<EventTableEntry>,
    checkpoints: Checkpoints,
    controller: Box<dyn FlushStrategy>,

    timers: LinkedList<Rc<RefCell<Timer>>>,
    transaction_to_timer: HashMap<TransactionId, Rc<RefCell<Timer>>>,

    slices: Vec<B::Output>,
    slice_backend_builder: B,
    slice_builder: SliceBuilder,
}

impl<B: SliceBackendBuilder> HostTransaction<B> {
    #[allow(dead_code)]
    pub fn new(
        event_table_capacity: usize,
        memory_table_capacity: usize,
        slice_backend_builder: B,
        controller: Box<dyn FlushStrategy>,
    ) -> Self {
        Self {
            event_table_capacity,
            memory_table_capacity,
            last_committed_event_cursor: 0,
            events: Vec::with_capacity(event_table_capacity * MAX_SLICES_IN_MEMORY),
            checkpoints: Checkpoints::new(event_table_capacity),
            controller,

            timers: LinkedList::new(),
            transaction_to_timer: HashMap::default(),

            slices: Vec::default(),
            slice_backend_builder,
            slice_builder: SliceBuilder::new(),
        }
    }

    fn _push_event(&mut self, event: EventTableEntry) {
        self.events.push(event);

        if self.events.len() == self.event_table_capacity * MAX_SLICES_IN_MEMORY {
            self.commit_slice();
        }
    }

    fn commit_slice(&mut self) {
        let limit_of_memory_table = {
            let mut last = self.events.len();
            let mut acc = 0;
            let mut init_set: HashSet<(specs::mtable::LocationType, u32)> = HashSet::default();

            for (index, event) in self.events.iter().enumerate() {
                for entry in memory_event_of_step(event).into_iter() {
                    let new = init_set.insert((entry.ltype, entry.offset));
                    if new {
                        acc += 1;
                    }

                    if entry.atype == AccessType::Write {
                        acc += 1;
                    }
                }

                if acc >= self.memory_table_capacity {
                    last = index;
                    break;
                }
            }

            last
        };

        // Find a checkpoint so that the size of the slice does not exceed capacity
        // return checkpoint, obliterated weak committed transactions
        let checkpoint = self.checkpoints.checkpoint(
            usize::min(
                usize::min(
                    self.last_committed_event_cursor + self.event_table_capacity,
                    self.last_committed_event_cursor + limit_of_memory_table,
                ),
                self.next_event_offset(),
            ),
            &*self.controller,
        );

        assert!(
            checkpoint != self.last_committed_event_cursor,
            "failed to select checkpoint"
        );

        // create slice
        let event_entries = self
            .events
            .drain(0..(checkpoint - self.last_committed_event_cursor))
            .collect();
        let slice = self.slice_builder.build(event_entries);
        self.slices.push(self.slice_backend_builder.build(slice));

        // reset
        self.last_committed_event_cursor = checkpoint;
    }

    fn next_event_offset(&self) -> usize {
        self.last_committed_event_cursor + self.events.len()
    }

    fn start(&mut self, tx: TransactionId) {
        let offset = self.next_event_offset();

        self.checkpoints.start(tx, offset);
    }

    fn commit(&mut self, tx: TransactionId, auto_finalize: bool) {
        let now = self.next_event_offset();

        let start = self.checkpoints.commit(tx, now);

        if now - start > self.event_table_capacity {
            panic!(
                "an overloaded transaction {} cannot be committed in a slice",
                tx
            );
        }

        if auto_finalize {
            self.start_timer(tx, start + self.event_table_capacity * TIMER_DELAY);
        }
    }

    fn finalize(&mut self, tx: TransactionId) {
        self.stop_timer(tx);
        self.checkpoints.finalize(tx);
    }

    fn finalize_all(&mut self) {
        let txs = self
            .transaction_to_timer
            .keys()
            .cloned()
            .collect::<Vec<_>>();

        for tx in txs {
            self.stop_timer(tx);
        }
        self.checkpoints.finalize_all();
    }

    fn start_timer(&mut self, tx: TransactionId, deadline: usize) {
        // stop previous timer if exists
        self.stop_timer(tx);

        let timer = Rc::new(RefCell::new(Timer {
            tx,
            deadline,
            disabled: false,
        }));

        self.timers.push_back(timer.clone());
        self.transaction_to_timer.insert(tx, timer);
    }

    fn stop_timer(&mut self, tx: TransactionId) {
        let timer = self.transaction_to_timer.remove(&tx);

        if let Some(timer) = timer {
            let mut timer = timer.borrow_mut();
            timer.disabled = true;
        }
    }

    fn tick(&mut self) {
        let mut delete = false;

        if let Some(timer) = self.timers.front() {
            let now = self.next_event_offset();
            let timer = timer.borrow();
            let tx = timer.tx;

            if timer.deadline != now {
                return;
            }

            delete = true;

            if !timer.disabled {
                drop(timer);

                warn!("Automatically commit overloaded weak transaction {}", tx);
                self.finalize(tx);
            }
        }

        if delete {
            self.timers.pop_front();
        }
    }
}

impl<B: SliceBackendBuilder> TransactionSlicer<B> for HostTransaction<B> {
    fn push_event(&mut self, event: EventTableEntry) {
        self.tick();

        let commands = match event.step_info {
            StepInfo::ExternalHostCall { op, value, .. } => {
                self.controller.notify(Event::HostCall(op, value))
            }
            _ => vec![Command::Noop],
        };

        for command in commands {
            match command {
                Command::Noop => self._push_event(event.clone()),
                Command::Start(tx) => {
                    self.start(tx);
                    self._push_event(event.clone());
                }
                Command::Commit(tx, timer) => {
                    self._push_event(event.clone());
                    self.commit(tx, timer);
                }
                Command::Abort => {
                    // V2 doesn't care abort from host
                }
                Command::Finalize(tx) => {
                    self.finalize(tx);
                }
            }
        }
    }

    fn finalize(mut self) -> Vec<B::Output> {
        self.finalize_all();

        while !self.events.is_empty() {
            self.commit_slice();
        }

        self.slices
    }

    fn frame_table_builder_get(&self) -> &FrameTableBuilder {
        &self.slice_builder.frame_table_builder
    }

    fn frame_table_builder_get_mut(&mut self) -> &mut FrameTableBuilder {
        &mut self.slice_builder.frame_table_builder
    }
}
