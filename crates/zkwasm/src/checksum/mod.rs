//use halo2_proofs::arithmetic::best_multiexp_gpu_cond;
use halo2_proofs::arithmetic::best_multiexp_gpu_cond;
use halo2_proofs::arithmetic::CurveAffine;
use halo2_proofs::arithmetic::FieldExt;
use halo2_proofs::poly::commitment::Params;
use num_bigint::BigUint;
use specs::brtable::BrTable;
use specs::brtable::ElemTable;
use specs::encode::image_table::ImageTableEncoder;
use specs::imtable::InitMemoryTable;
use specs::itable::InstructionTable;
use specs::jtable::StaticFrameEntry;
use specs::mtable::LocationType;
use specs::CompilationTable;

use crate::circuits::config::max_image_table_rows;
use crate::circuits::utils::bn_to_field;

pub trait ImageCheckSum<Output> {
    fn checksum(&self) -> Output;
}

pub(crate) struct CompilationTableWithParams<'a, 'b, C: CurveAffine> {
    pub(crate) table: &'a CompilationTable,
    pub(crate) params: &'b Params<C>,
}

impl<'a, 'b, C: CurveAffine> ImageCheckSum<Vec<C>> for CompilationTableWithParams<'a, 'b, C> {
    fn checksum(&self) -> Vec<C> {
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

        let mut cells: Vec<C::ScalarExt> = vec![];

        cells.append(&mut msg_of_image_table(
            &self.table.itable,
            &self.table.itable.create_brtable(),
            &self.table.elem_table,
            &self.table.imtable,
        ));

        cells.push(C::ScalarExt::from(self.table.fid_of_entry as u64));
        cells.append(&mut msg_of_static_frame_table(&self.table.static_jtable));

        let c = best_multiexp_gpu_cond(&cells[..], &self.params.get_g_lagrange()[0..cells.len()]);
        vec![c.into()]
    }
}
