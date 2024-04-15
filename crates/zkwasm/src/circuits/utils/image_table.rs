use anyhow::Error;
use halo2_proofs::arithmetic::FieldExt;
use num_bigint::BigUint;
use specs::brtable::BrTable;
use specs::brtable::ElemTable;
use specs::encode::image_table::ImageTableEncoder;
use specs::imtable::InitMemoryTable;
use specs::imtable::InitMemoryTableEntry;
use specs::itable::InstructionTable;
use specs::jtable::StaticFrameEntry;
use specs::jtable::STATIC_FRAME_ENTRY_NUMBER;
use specs::mtable::LocationType;
use specs::mtable::VarType;
use specs::slice::Slice;
use specs::state::InitializationState;
use wasmi::DEFAULT_VALUE_STACK_LIMIT;

use crate::circuits::image_table::compute_maximal_pages;
use crate::circuits::image_table::PAGE_ENTRIES;
use crate::circuits::jtable::STATIC_FRAME_ENTRY_IMAGE_TABLE_ENTRY;
use crate::circuits::utils::bn_to_field;

pub const STACK_CAPABILITY: usize = DEFAULT_VALUE_STACK_LIMIT;
pub const GLOBAL_CAPABILITY: usize = DEFAULT_VALUE_STACK_LIMIT;
pub const INIT_MEMORY_ENTRIES_OFFSET: usize = 40960;

pub(crate) struct InitMemoryLayouter {
    pub(crate) pages: u32,
}

impl InitMemoryLayouter {
    fn for_each(self, mut f: impl FnMut((LocationType, u32))) {
        for offset in 0..STACK_CAPABILITY {
            f((LocationType::Stack, offset as u32))
        }

        for offset in 0..GLOBAL_CAPABILITY {
            f((LocationType::Global, offset as u32))
        }

        for offset in 0..(self.pages * PAGE_ENTRIES) {
            f((LocationType::Heap, offset))
        }
    }
}

pub fn image_table_offset_to_memory_location(offset: usize) -> (LocationType, u32) {
    // Minus one for default lookup entry.
    let mut offset = offset - INIT_MEMORY_ENTRIES_OFFSET - 1;

    if offset < STACK_CAPABILITY {
        return (LocationType::Stack, offset as u32);
    }

    offset -= STACK_CAPABILITY;

    if offset < GLOBAL_CAPABILITY {
        return (LocationType::Global, offset as u32);
    }

    offset -= GLOBAL_CAPABILITY;
    return (LocationType::Heap, offset as u32);
}

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
 * -------------------- Init Memory Offset(Constant INIT_MEMORY_ENTRIES_OFFSET)
 * Stack
 * --------------------
 * Global
 * --------------------
 * Heap
 * --------------------
 */
#[derive(Debug)]
pub struct ImageTableLayouter<T> {
    pub(crate) initialization_state: InitializationState<T>,
    pub(crate) static_frame_entries: [(T, T); STATIC_FRAME_ENTRY_NUMBER],
    pub(crate) instructions: Vec<T>,
    pub(crate) br_table_entires: Vec<T>,
    // NOTE: unused instructions and br_table entries.
    pub(crate) padding_entires: Vec<T>,
    pub(crate) init_memory_entries: Vec<T>,
}

#[derive(Clone, Copy)]
pub struct ImageTableAssigner {
    pub heap_capability: u32,

    initialization_state_offset: usize,
    static_frame_entries_offset: usize,
    instruction_offset: usize,
    br_table_offset: usize,
    padding_offset: usize,
    init_memory_offset: usize,
}

impl ImageTableAssigner {
    /// `instruction_number` and `br_table_number` came from wasm image. Instructions, br table entries and paddings
    /// are compacted within a fixed range. `page_capability` is computed based on K.
    pub fn new(instruction_number: usize, br_table_number: usize, pages_capability: u32) -> Self {
        let initialization_state_offset = 0;
        let static_frame_entries_offset =
            initialization_state_offset + InitializationState::<u32>::field_count();
        let instruction_offset = static_frame_entries_offset + STATIC_FRAME_ENTRY_IMAGE_TABLE_ENTRY;
        let br_table_offset = instruction_offset + instruction_number;
        let padding_offset = br_table_offset + br_table_number;
        let init_memory_offset = INIT_MEMORY_ENTRIES_OFFSET;

        assert!(
            padding_offset <= init_memory_offset,
            "The number of instructions of the image({}) is too large",
            instruction_number + br_table_number
        );

        Self {
            heap_capability: pages_capability * PAGE_ENTRIES,

            initialization_state_offset,
            static_frame_entries_offset,
            instruction_offset,
            br_table_offset,
            padding_offset,
            init_memory_offset,
        }
    }

    pub fn exec_initialization_state<T, Error>(
        &self,
        mut initialization_state_handler: impl FnMut(usize) -> Result<InitializationState<T>, Error>,
    ) -> Result<InitializationState<T>, Error> {
        initialization_state_handler(self.initialization_state_offset)
    }

    pub fn exec_static_frame_entries<T, Error>(
        &self,
        mut static_frame_entries_handler: impl FnMut(
            usize,
        ) -> Result<
            [(T, T); STATIC_FRAME_ENTRY_NUMBER],
            Error,
        >,
    ) -> Result<[(T, T); STATIC_FRAME_ENTRY_NUMBER], Error> {
        static_frame_entries_handler(self.static_frame_entries_offset)
    }

    pub fn exec_instruction<T, Error>(
        &self,
        mut instruction_handler: impl FnMut(usize) -> Result<Vec<T>, Error>,
    ) -> Result<Vec<T>, Error> {
        instruction_handler(self.instruction_offset)
    }

    pub fn exec_br_table_entires<T, Error>(
        &self,
        mut br_table_handler: impl FnMut(usize) -> Result<Vec<T>, Error>,
    ) -> Result<Vec<T>, Error> {
        br_table_handler(self.br_table_offset)
    }

    pub fn exec_padding_entires<T, Error>(
        &self,
        mut padding_handler: impl FnMut(usize, usize) -> Result<Vec<T>, Error>,
    ) -> Result<Vec<T>, Error> {
        padding_handler(self.padding_offset, self.init_memory_offset)
    }

    pub fn exec_init_memory_entries<T, Error>(
        &self,
        mut init_memory_entries_handler: impl FnMut(usize) -> Result<Vec<T>, Error>,
    ) -> Result<Vec<T>, Error> {
        init_memory_entries_handler(self.init_memory_offset)
    }

    pub fn exec<T, Error>(
        &self,
        initialization_state_handler: impl FnMut(usize) -> Result<InitializationState<T>, Error>,
        static_frame_entries_handler: impl FnMut(
            usize,
        )
            -> Result<[(T, T); STATIC_FRAME_ENTRY_NUMBER], Error>,
        instruction_handler: impl FnMut(usize) -> Result<Vec<T>, Error>,
        br_table_handler: impl FnMut(usize) -> Result<Vec<T>, Error>,
        padding_handler: impl FnMut(usize, usize) -> Result<Vec<T>, Error>,
        init_memory_entries_handler: impl FnMut(usize) -> Result<Vec<T>, Error>,
    ) -> Result<ImageTableLayouter<T>, Error> {
        let initialization_state = self.exec_initialization_state(initialization_state_handler)?;
        let static_frame_entries = self.exec_static_frame_entries(static_frame_entries_handler)?;
        let instructions = self.exec_instruction(instruction_handler)?;
        let br_table_entires = self.exec_br_table_entires(br_table_handler)?;
        let padding_entires = self.exec_padding_entires(padding_handler)?;
        let init_memory_entries = self.exec_init_memory_entries(init_memory_entries_handler)?;

        Ok(ImageTableLayouter {
            initialization_state,
            static_frame_entries,
            instructions,
            br_table_entires,
            padding_entires,
            init_memory_entries,
        })
    }
}

pub(crate) fn encode_compilation_table_values<F: FieldExt>(
    k: u32,
    itable: &InstructionTable,
    br_table: &BrTable,
    elem_table: &ElemTable,
    static_frame_entries: &[StaticFrameEntry; STATIC_FRAME_ENTRY_NUMBER],
    initialization_state: &InitializationState<u32>,
    init_memory_table: &InitMemoryTable,
) -> ImageTableLayouter<F> {
    let page_capability = compute_maximal_pages(k);

    let initialization_state_handler = |_| Ok(initialization_state.map(|v| F::from((*v) as u64)));

    let static_frame_entries_handler = |_| {
        let mut cells = vec![];

        for entry in static_frame_entries.as_ref() {
            cells.push((F::from(entry.enable as u64), bn_to_field(&entry.encode())));
        }

        Ok(cells.try_into().expect(&format!(
            "The number of static frame entries should be {}",
            STATIC_FRAME_ENTRY_NUMBER
        )))
    };

    let instruction_handler = |_| {
        let mut cells = vec![];

        cells.push(bn_to_field(
            &ImageTableEncoder::Instruction.encode(BigUint::from(0u64)),
        ));

        for e in itable.iter() {
            cells.push(bn_to_field(
                &ImageTableEncoder::Instruction.encode(e.encode.clone()),
            ));
        }

        Ok(cells)
    };

    let br_table_handler = |_| {
        let mut cells = vec![];

        cells.push(bn_to_field(
            &ImageTableEncoder::BrTable.encode(BigUint::from(0u64)),
        ));

        for e in br_table.entries() {
            cells.push(bn_to_field(&ImageTableEncoder::BrTable.encode(e.encode())));
        }

        for e in elem_table.entries() {
            cells.push(bn_to_field(&ImageTableEncoder::BrTable.encode(e.encode())));
        }

        Ok(cells)
    };

    let padding_handler = |start, end| Ok(vec![F::zero(); end - start]);

    let init_memory_entries_handler = |_| {
        let mut cells = vec![];

        cells.push(bn_to_field(
            &ImageTableEncoder::InitMemory.encode(BigUint::from(0u64)),
        ));

        let layouter = InitMemoryLayouter {
            pages: page_capability,
        };

        layouter.for_each(|(ltype, offset)| {
            if let Some(entry) = init_memory_table.try_find(ltype, offset) {
                cells.push(bn_to_field::<F>(
                    &ImageTableEncoder::InitMemory.encode(entry.encode()),
                ));
            } else if ltype == LocationType::Heap {
                let entry = InitMemoryTableEntry {
                    ltype,
                    is_mutable: true,
                    offset,
                    vtype: VarType::I64,
                    value: 0,
                    eid: 0,
                };

                cells.push(bn_to_field::<F>(
                    &ImageTableEncoder::InitMemory.encode(entry.encode()),
                ));
            } else {
                cells.push(bn_to_field::<F>(
                    &ImageTableEncoder::InitMemory.encode(BigUint::from(0u64)),
                ));
            }
        });

        Ok(cells)
    };

    let assigner = ImageTableAssigner::new(
        itable.len() + 1,
        br_table.entries().len() + elem_table.entries().len() + 1,
        page_capability,
    );

    let layouter = assigner
        .exec::<_, Error>(
            initialization_state_handler,
            static_frame_entries_handler,
            instruction_handler,
            br_table_handler,
            padding_handler,
            init_memory_entries_handler,
        )
        .unwrap();

    layouter
}

pub(crate) trait EncodeImageTable<F: FieldExt> {
    fn encode_pre_compilation_table_values(&self, k: u32) -> ImageTableLayouter<F>;

    fn encode_post_compilation_table_values(&self, k: u32) -> ImageTableLayouter<F>;
}

impl<F: FieldExt> EncodeImageTable<F> for Slice {
    fn encode_pre_compilation_table_values(&self, k: u32) -> ImageTableLayouter<F> {
        encode_compilation_table_values(
            k,
            &self.itable,
            &self.br_table,
            &self.elem_table,
            &self.static_jtable,
            &self.initialization_state,
            &self.imtable,
        )
    }

    fn encode_post_compilation_table_values(&self, k: u32) -> ImageTableLayouter<F> {
        encode_compilation_table_values(
            k,
            &self.itable,
            &self.br_table,
            &self.elem_table,
            &self.static_jtable,
            &self.post_initialization_state,
            &self.post_imtable,
        )
    }
}

impl<F: FieldExt> ImageTableLayouter<F> {
    pub fn plain(&self) -> Vec<F> {
        let mut buf = vec![];

        buf.append(&mut self.initialization_state.plain());
        buf.append(
            &mut self
                .static_frame_entries
                .map(|(enable, fid)| vec![enable, fid])
                .into_iter()
                .collect::<Vec<Vec<_>>>()
                .concat(),
        );
        buf.append(&mut self.instructions.clone());
        buf.append(&mut self.br_table_entires.clone());
        buf.append(&mut self.padding_entires.clone());
        buf.append(&mut self.init_memory_entries.clone());

        buf
    }
}
