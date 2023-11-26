use specs::state::InitializationState;

use crate::circuits::image_table::PAGE_ENTRIES;

/*
 * --------------------
 * Initialization State
 * --------------------
 * Static Frame Entries
 * --------------------
 * Instructions
 * --------------------
 * Br Table
 * --------------------
 * Padding
 * -------------------- Init Memory Offset
 * Stack
 * --------------------
 * Global
 * --------------------
 * Heap
 * --------------------
 */
pub(crate) struct Layouter<T> {
    pub(crate) initialization_state: InitializationState<T>,
    pub(crate) static_frame_entries: Vec<(T, T)>,
    pub(crate) instructions: Vec<T>,
    pub(crate) br_table_entires: Vec<T>,
    // NOTE: padding entries also need constain_equal for other image
    pub(crate) _padding_entires: Vec<T>,
    pub(crate) _init_memory_entires: Vec<T>,
}

pub(crate) struct ImageTableAssigner<
    const INIT_MEMORY_OFFSET: usize,
    const STACK_CAPABILITY: usize,
    const GLOBAL_CAPABILITY: usize,
> {
    _heap_capability: u32,
    initialization_state_offset: usize,
    static_frame_entries_offset: usize,
    instruction_offset: usize,
    br_table_offset: usize,
    padding_offset: usize,
    init_memory_offset: usize,
}

impl<
        const INIT_MEMORY_OFFSET: usize,
        const STACK_CAPABILITY: usize,
        const GLOBAL_CAPABILITY: usize,
    > ImageTableAssigner<INIT_MEMORY_OFFSET, STACK_CAPABILITY, GLOBAL_CAPABILITY>
{
    pub fn new(instruction_number: usize, br_table_number: usize, pages_capability: u32) -> Self {
        let initialization_state_offset = 0;
        let static_frame_entries_offset =
            initialization_state_offset + InitializationState::<u32>::field_count();
        // FIXME: magic number
        let instruction_offset = static_frame_entries_offset + 4;
        let br_table_offset = instruction_offset + instruction_number;
        let padding_offset = br_table_offset + br_table_number;
        let init_memory_offset = INIT_MEMORY_OFFSET;

        assert!(padding_offset <= init_memory_offset);

        Self {
            _heap_capability: pages_capability * PAGE_ENTRIES,
            initialization_state_offset,
            static_frame_entries_offset,
            instruction_offset,
            br_table_offset,
            padding_offset,
            init_memory_offset,
        }
    }

    pub fn exec_initialization_state<T, Error>(
        &mut self,
        mut initialization_state_handler: impl FnMut(usize) -> Result<InitializationState<T>, Error>,
    ) -> Result<InitializationState<T>, Error> {
        initialization_state_handler(self.initialization_state_offset)
    }

    pub fn exec_static_frame_entries<T, Error>(
        &mut self,
        mut static_frame_entries_handler: impl FnMut(usize) -> Result<Vec<(T, T)>, Error>,
    ) -> Result<Vec<(T, T)>, Error> {
        static_frame_entries_handler(self.static_frame_entries_offset)
    }

    pub fn exec_instruction<T, Error>(
        &mut self,
        mut instruction_handler: impl FnMut(usize) -> Result<Vec<T>, Error>,
    ) -> Result<Vec<T>, Error> {
        instruction_handler(self.instruction_offset)
    }

    pub fn exec_br_table_entires<T, Error>(
        &mut self,
        mut br_table_handler: impl FnMut(usize) -> Result<Vec<T>, Error>,
    ) -> Result<Vec<T>, Error> {
        br_table_handler(self.br_table_offset)
    }

    pub fn exec_padding_entires<T, Error>(
        &mut self,
        mut padding_handler: impl FnMut(usize, usize) -> Result<Vec<T>, Error>,
    ) -> Result<Vec<T>, Error> {
        padding_handler(self.padding_offset, self.padding_offset)
    }

    pub fn exec_init_memory_entires<T, Error>(
        &mut self,
        mut init_memory_entries_handler: impl FnMut(usize) -> Result<Vec<T>, Error>,
    ) -> Result<Vec<T>, Error> {
        init_memory_entries_handler(self.init_memory_offset)
    }

    pub fn exec<T, Error>(
        &mut self,
        initialization_state_handler: impl FnMut(usize) -> Result<InitializationState<T>, Error>,
        static_frame_entries_handler: impl FnMut(usize) -> Result<Vec<(T, T)>, Error>,
        instruction_handler: impl FnMut(usize) -> Result<Vec<T>, Error>,
        br_table_handler: impl FnMut(usize) -> Result<Vec<T>, Error>,
        padding_handler: impl FnMut(usize, usize) -> Result<Vec<T>, Error>,
        init_memory_entries_handler: impl FnMut(usize) -> Result<Vec<T>, Error>,
    ) -> Result<Layouter<T>, Error> {
        let initialization_state = self.exec_initialization_state(initialization_state_handler)?;
        let static_frame_entries = self.exec_static_frame_entries(static_frame_entries_handler)?;
        let instructions = self.exec_instruction(instruction_handler)?;
        let br_table_entires = self.exec_br_table_entires(br_table_handler)?;
        let _padding_entires = self.exec_padding_entires(padding_handler)?;
        let _init_memory_entires = self.exec_init_memory_entires(init_memory_entries_handler)?;

        Ok(Layouter {
            initialization_state,
            static_frame_entries,
            instructions,
            br_table_entires,
            _padding_entires,
            _init_memory_entires,
        })
    }
}
