use halo2_proofs::arithmetic::FieldExt;
use halo2_proofs::circuit::Layouter;
use halo2_proofs::plonk::Error;
use num_bigint::BigUint;
use specs::brtable::BrTable;
use specs::brtable::ElemTable;
use specs::encode::image_table::ImageTableEncoder;
use specs::imtable::InitMemoryTable;
use specs::itable::InstructionTable;
use specs::mtable::LocationType;

use crate::circuits::utils::bn_to_field;

use super::ImageTableChip;

impl<F: FieldExt> ImageTableChip<F> {
    pub fn assign(
        self,
        layouter: &mut impl Layouter<F>,
        instructions: &InstructionTable,
        br_table: &BrTable,
        elem_table: &ElemTable,
        init_memory_table: &InitMemoryTable,
    ) -> Result<(), Error> {
        layouter.assign_table(
            || "image table",
            |mut table| {
                let mut offset = 0;

                {
                    table.assign_cell(
                        || "instruction table",
                        self.config.col,
                        offset,
                        || {
                            Ok(bn_to_field::<F>(
                                &ImageTableEncoder::Instruction.encode(BigUint::from(0u64)),
                            ))
                        },
                    )?;

                    offset += 1;

                    for e in instructions.entries().iter() {
                        table.assign_cell(
                            || "instruction table",
                            self.config.col,
                            offset,
                            || {
                                Ok(bn_to_field::<F>(
                                    &ImageTableEncoder::Instruction.encode(e.encode()),
                                ))
                            },
                        )?;

                        offset += 1;
                    }
                }

                {
                    table.assign_cell(
                        || "br table empty cell",
                        self.config.col,
                        offset,
                        || {
                            Ok(bn_to_field::<F>(
                                &ImageTableEncoder::BrTable.encode(BigUint::from(0u64)),
                            ))
                        },
                    )?;

                    offset += 1;

                    for e in br_table.entries() {
                        table.assign_cell(
                            || "br table init cell",
                            self.config.col,
                            offset,
                            || {
                                Ok(bn_to_field::<F>(
                                    &ImageTableEncoder::BrTable.encode(e.encode()),
                                ))
                            },
                        )?;

                        offset += 1;
                    }

                    for e in elem_table.entries() {
                        table.assign_cell(
                            || "call indirect init cell",
                            self.config.col,
                            offset,
                            || {
                                Ok(bn_to_field::<F>(
                                    &ImageTableEncoder::BrTable.encode(e.encode()),
                                ))
                            },
                        )?;

                        offset += 1;
                    }
                }

                {
                    let heap_entries = init_memory_table.filter(LocationType::Heap);
                    let global_entries = init_memory_table.filter(LocationType::Global);

                    table.assign_cell(
                        || "init memory table empty",
                        self.config.col,
                        offset,
                        || {
                            Ok(bn_to_field::<F>(
                                &ImageTableEncoder::InitMemory.encode(BigUint::from(0u64)),
                            ))
                        },
                    )?;

                    offset += 1;

                    for v in heap_entries.into_iter().chain(global_entries.into_iter()) {
                        table.assign_cell(
                            || "init memory table cell",
                            self.config.col,
                            offset,
                            || {
                                Ok(bn_to_field::<F>(
                                    &ImageTableEncoder::InitMemory.encode(v.encode()),
                                ))
                            },
                        )?;

                        offset += 1;
                    }
                }

                Ok(())
            },
        )
    }
}
