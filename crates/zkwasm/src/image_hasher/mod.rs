use halo2_proofs::arithmetic::FieldExt;
use num_bigint::BigUint;
use specs::brtable::BrTable;
use specs::brtable::ElemTable;
use specs::encode::image_table::ImageTableEncoder;
use specs::imtable::InitMemoryTable;
use specs::itable::InstructionTable;
use specs::jtable::StaticFrameEntry;
use specs::mtable::LocationType;
use specs::CompilationTable;

use crate::circuits::checksum::poseidon::primitives::ConstantLength;
use crate::circuits::checksum::poseidon::primitives::Hash;
use crate::circuits::checksum::poseidon::primitives::P128Pow5T9;
use crate::circuits::checksum::L;
use crate::circuits::config::max_image_table_rows;
use crate::circuits::utils::bn_to_field;

pub trait ImageHasher {
    fn hash<F: FieldExt>(&self) -> F;
}

impl ImageHasher for CompilationTable {
    fn hash<F: FieldExt>(&self) -> F {
        fn msg_of_instruction_table<F: FieldExt>(instruction_table: &InstructionTable) -> Vec<F> {
            let mut cells = vec![];

            cells.push(bn_to_field(
                &ImageTableEncoder::Instruction.encode(BigUint::from(0u64)),
            ));

            for e in instruction_table.entries() {
                cells.push(bn_to_field(
                    &ImageTableEncoder::Instruction.encode(e.encode()),
                ));
            }

            cells
        }

        fn msg_of_br_table<F: FieldExt>(br_table: &BrTable, elem_table: &ElemTable) -> Vec<F> {
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

            cells
        }

        fn msg_of_init_memory_table<F: FieldExt>(init_memory_table: &InitMemoryTable) -> Vec<F> {
            let heap_entries = init_memory_table.filter(LocationType::Heap);
            let global_entries = init_memory_table.filter(LocationType::Global);

            let mut cells = vec![];

            cells.push(bn_to_field(
                &ImageTableEncoder::InitMemory.encode(BigUint::from(0u64)),
            ));

            for v in heap_entries.into_iter().chain(global_entries.into_iter()) {
                cells.push(bn_to_field::<F>(
                    &ImageTableEncoder::InitMemory.encode(v.encode()),
                ));
            }

            cells
        }

        fn msg_of_image_table<F: FieldExt>(
            instruction_table: &InstructionTable,
            br_table: &BrTable,
            elem_table: &ElemTable,
            init_memory_table: &InitMemoryTable,
        ) -> Vec<F> {
            let mut cells = vec![];

            cells.append(&mut msg_of_instruction_table(instruction_table));
            cells.append(&mut msg_of_br_table(br_table, elem_table));
            cells.append(&mut msg_of_init_memory_table(init_memory_table));

            for _ in cells.len()..(max_image_table_rows() as usize) {
                cells.push(F::zero());
            }

            cells
        }

        fn msg_of_static_frame_table<F: FieldExt>(
            static_frame_table: &Vec<StaticFrameEntry>,
        ) -> Vec<F> {
            let mut cells = static_frame_table
                .into_iter()
                .map(|entry| vec![F::one(), bn_to_field(&entry.encode())])
                .collect::<Vec<Vec<_>>>();

            cells.resize(
                2,
                vec![
                    F::zero(),
                    bn_to_field(
                        &StaticFrameEntry {
                            frame_id: 0,
                            next_frame_id: 0,
                            callee_fid: 0,
                            fid: 0,
                            iid: 0,
                        }
                        .encode(),
                    ),
                ],
            );

            cells.concat()
        }

        let mut cells: Vec<F> = vec![];

        cells.append(&mut msg_of_image_table(
            &self.itable,
            &self.itable.create_brtable(),
            &self.elem_table,
            &self.imtable,
        ));

        cells.push(F::from(self.fid_of_entry as u64));
        cells.append(&mut msg_of_static_frame_table(&self.static_jtable));

        let poseidon_hasher = Hash::<F, P128Pow5T9<F>, ConstantLength<L>, 9, 8>::init();
        poseidon_hasher.hash(cells.try_into().unwrap())
    }
}
