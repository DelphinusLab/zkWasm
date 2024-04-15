use std::collections::HashSet;
use std::sync::Arc;

use rayon::iter::IntoParallelRefIterator;
use rayon::iter::ParallelIterator;

use crate::brtable::BrTable;
use crate::brtable::ElemTable;
use crate::configure_table::ConfigureTable;
use crate::etable::EventTable;
use crate::etable::EventTableEntry;
use crate::imtable::InitMemoryTable;
use crate::itable::InstructionTable;
use crate::jtable::JumpTable;
use crate::mtable::AccessType;
use crate::mtable::LocationType;
use crate::mtable::MTable;
use crate::mtable::MemoryTableEntry;
use crate::state::InitializationState;
use crate::CompilationTable;
use crate::StaticFrameEntry;
use crate::STATIC_FRAME_ENTRY_NUMBER;

pub struct Slice {
    pub itable: Arc<InstructionTable>,
    pub br_table: Arc<BrTable>,
    pub elem_table: Arc<ElemTable>,
    pub configure_table: Arc<ConfigureTable>,
    pub static_jtable: Arc<[StaticFrameEntry; STATIC_FRAME_ENTRY_NUMBER]>,

    pub etable: Arc<EventTable>,
    pub frame_table: Arc<JumpTable>,

    pub imtable: Arc<InitMemoryTable>,
    pub post_imtable: Arc<InitMemoryTable>,

    pub initialization_state: Arc<InitializationState<u32>>,
    pub post_initialization_state: Arc<InitializationState<u32>>,

    pub is_last_slice: bool,
}

impl Slice {
    pub fn from_compilation_table(
        compilation_table: &CompilationTable,
        is_last_slice: bool,
    ) -> Self {
        Slice {
            itable: compilation_table.itable.clone(),
            br_table: compilation_table.br_table.clone(),
            elem_table: compilation_table.elem_table.clone(),
            configure_table: compilation_table.configure_table.clone(),
            static_jtable: compilation_table.static_jtable.clone(),

            etable: EventTable::default().into(),
            frame_table: JumpTable::default().into(),

            imtable: compilation_table.imtable.clone(),
            post_imtable: compilation_table.imtable.clone(),

            initialization_state: compilation_table.initialization_state.clone(),
            post_initialization_state: compilation_table.initialization_state.clone(),

            is_last_slice,
        }
    }

    pub fn create_memory_table(
        &self,
        memory_event_of_step: fn(&EventTableEntry) -> Vec<MemoryTableEntry>,
    ) -> MTable {
        let mut memory_entries = self
            .etable
            .entries()
            .par_iter()
            .map(|entry| memory_event_of_step(entry))
            .collect::<Vec<Vec<_>>>()
            .concat();

        let mut set = HashSet::<MemoryTableEntry>::default();

        memory_entries.iter().for_each(|entry| {
            let init_memory_entry = self.imtable.try_find(entry.ltype, entry.offset);

            if let Some(init_memory_entry) = init_memory_entry {
                set.insert(MemoryTableEntry {
                    eid: init_memory_entry.eid,
                    offset: entry.offset,
                    ltype: entry.ltype,
                    atype: AccessType::Init,
                    vtype: init_memory_entry.vtype,
                    is_mutable: entry.is_mutable,
                    value: init_memory_entry.value,
                });
            } else if entry.ltype == LocationType::Heap {
                // Heap value without init memory entry should equal 0
                set.insert(MemoryTableEntry {
                    eid: 0,
                    offset: entry.offset,
                    ltype: entry.ltype,
                    atype: AccessType::Init,
                    vtype: entry.vtype,
                    is_mutable: entry.is_mutable,
                    value: 0,
                });
            }
        });

        memory_entries.append(&mut set.into_iter().collect());

        memory_entries.sort_by_key(|item| (item.ltype, item.offset, item.eid));

        MTable::new(memory_entries)
    }
}
