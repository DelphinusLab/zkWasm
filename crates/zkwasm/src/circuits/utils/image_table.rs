use anyhow::Error;
use halo2_proofs::arithmetic::FieldExt;
use num_bigint::BigUint;
use specs::encode::image_table::ImageTableEncoder;
use specs::imtable::InitMemoryTableEntry;
use specs::jtable::STATIC_FRAME_ENTRY_NUMBER;
use specs::mtable::LocationType;
use specs::mtable::VarType;
use specs::state::InitializationState;
use specs::CompilationTable;
use wasmi::DEFAULT_VALUE_STACK_LIMIT;

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
#[allow(dead_code)]
pub(crate) struct ImageTableLayouter<T> {
    pub(crate) initialization_state: InitializationState<T>,
    pub(crate) static_frame_entries: [(T, T); STATIC_FRAME_ENTRY_NUMBER],
    pub(crate) instructions: Vec<T>,
    pub(crate) br_table_entires: Vec<T>,
    // NOTE: unused instructions and br_table entries.
    pub(crate) padding_entires: Vec<T>,
    pub(crate) init_memory_entries: Vec<T>,
}

pub(crate) struct ImageTableAssigner {
    pub(crate) heap_capability: u32,

    initialization_state_offset: usize,
    static_frame_entries_offset: usize,
    instruction_offset: usize,
    br_table_offset: usize,
    padding_offset: usize,
    init_memory_offset: usize,
}

impl ImageTableAssigner {
    pub fn new(instruction_number: usize, br_table_number: usize, pages_capability: u32) -> Self {
        let initialization_state_offset = 0;
        let static_frame_entries_offset =
            initialization_state_offset + InitializationState::<u32>::field_count();
        let instruction_offset = static_frame_entries_offset + STATIC_FRAME_ENTRY_IMAGE_TABLE_ENTRY;
        let br_table_offset = instruction_offset + instruction_number;
        let padding_offset = br_table_offset + br_table_number;
        let init_memory_offset = INIT_MEMORY_ENTRIES_OFFSET;

        assert!(padding_offset <= init_memory_offset);

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
        &mut self,
        mut initialization_state_handler: impl FnMut(usize) -> Result<InitializationState<T>, Error>,
    ) -> Result<InitializationState<T>, Error> {
        initialization_state_handler(self.initialization_state_offset)
    }

    pub fn exec_static_frame_entries<T, Error>(
        &mut self,
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

    pub fn exec_init_memory_entries<T, Error>(
        &mut self,
        mut init_memory_entries_handler: impl FnMut(usize) -> Result<Vec<T>, Error>,
    ) -> Result<Vec<T>, Error> {
        init_memory_entries_handler(self.init_memory_offset)
    }

    pub fn exec<T, Error>(
        &mut self,
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

pub(crate) trait EncodeCompilationTableValues<F: Clone> {
    fn encode_compilation_table_values(&self, page_capability: u32) -> ImageTableLayouter<F>;
}

impl<F: FieldExt> EncodeCompilationTableValues<F> for CompilationTable {
    fn encode_compilation_table_values(&self, page_capability: u32) -> ImageTableLayouter<F> {
        let initialization_state_handler =
            |_| Ok(self.initialization_state.map(|v| F::from((*v) as u64)));

        let static_frame_entries_handler = |_| {
            // Encode disabled static frame entry in image table
            assert_eq!(self.static_jtable.len(), STATIC_FRAME_ENTRY_NUMBER);

            let mut cells = vec![];

            for entry in self.static_jtable.as_ref() {
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

            for e in self.itable.entries() {
                cells.push(bn_to_field(
                    &ImageTableEncoder::Instruction.encode(e.encode()),
                ));
            }

            Ok(cells)
        };

        let br_table_handler = |_| {
            let mut cells = vec![];

            cells.push(bn_to_field(
                &ImageTableEncoder::BrTable.encode(BigUint::from(0u64)),
            ));

            for e in self.br_table.entries() {
                cells.push(bn_to_field(&ImageTableEncoder::BrTable.encode(e.encode())));
            }

            for e in self.elem_table.entries() {
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
                if let Some(entry) = self.imtable.try_find(ltype, offset) {
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

        let mut assigner = ImageTableAssigner::new(
            self.itable.entries().len(),
            self.br_table.entries().len() + self.elem_table.entries().len(),
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
