use halo2_proofs::arithmetic::FieldExt;
use halo2_proofs::circuit::AssignedCell;
use halo2_proofs::circuit::Layouter;
use halo2_proofs::plonk::Error;
use num_bigint::BigUint;
use specs::brtable::BrTable;
use specs::brtable::ElemTable;
use specs::encode::image_table::ImageTableEncoder;
use specs::imtable::InitMemoryTable;
use specs::itable::InstructionTable;
use specs::mtable::LocationType;

use crate::circuits::config::max_image_table_rows;
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
    ) -> Result<Vec<AssignedCell<F, F>>, Error> {
        layouter.assign_region(
            || "image table",
            |mut table| {
                let mut ret = vec![];
                let mut offset = 0;

                {
                    let cell = table.assign_advice(
                        || "instruction table",
                        self.config.col,
                        offset,
                        || {
                            Ok(bn_to_field::<F>(
                                &ImageTableEncoder::Instruction.encode(BigUint::from(0u64)),
                            ))
                        },
                    )?;

                    ret.push(cell);
                    offset += 1;

                    for e in instructions.entries().iter() {
                        let cell = table.assign_advice(
                            || "instruction table",
                            self.config.col,
                            offset,
                            || {
                                Ok(bn_to_field::<F>(
                                    &ImageTableEncoder::Instruction.encode(e.encode()),
                                ))
                            },
                        )?;

                        ret.push(cell);
                        offset += 1;
                    }
                }

                {
                    let cell = table.assign_advice(
                        || "br table init cell",
                        self.config.col,
                        offset,
                        || {
                            Ok(bn_to_field::<F>(
                                &ImageTableEncoder::BrTable.encode(BigUint::from(0u64)),
                            ))
                        },
                    )?;

                    ret.push(cell);
                    offset += 1;

                    for e in br_table.entries() {
                        let cell = table.assign_advice(
                            || "br table init cell",
                            self.config.col,
                            offset,
                            || {
                                Ok(bn_to_field::<F>(
                                    &ImageTableEncoder::BrTable.encode(e.encode()),
                                ))
                            },
                        )?;

                        ret.push(cell);
                        offset += 1;
                    }

                    for e in elem_table.entries() {
                        let cell = table.assign_advice(
                            || "call indirect init cell",
                            self.config.col,
                            offset,
                            || {
                                Ok(bn_to_field::<F>(
                                    &ImageTableEncoder::BrTable.encode(e.encode()),
                                ))
                            },
                        )?;

                        ret.push(cell);
                        offset += 1;
                    }
                }

                {
                    let heap_entries = init_memory_table.filter(LocationType::Heap);
                    let global_entries = init_memory_table.filter(LocationType::Global);

                    let cell = table.assign_advice(
                        || "br table init cell",
                        self.config.col,
                        offset,
                        || {
                            Ok(bn_to_field::<F>(
                                &ImageTableEncoder::InitMemory.encode(BigUint::from(0u64)),
                            ))
                        },
                    )?;

                    ret.push(cell);
                    offset += 1;

                    for v in heap_entries.into_iter().chain(global_entries.into_iter()) {
                        let cell = table.assign_advice(
                            || "init memory table cell",
                            self.config.col,
                            offset,
                            || {
                                Ok(bn_to_field::<F>(
                                    &ImageTableEncoder::InitMemory.encode(v.encode()),
                                ))
                            },
                        )?;

                        ret.push(cell);
                        offset += 1;
                    }
                }

                {
                    let max_rows = max_image_table_rows() as usize;
                    assert!(offset < max_rows);

                    while offset < max_rows {
                        let cell = table.assign_advice(
                            || "image table padding",
                            self.config.col,
                            offset,
                            || Ok(F::zero()),
                        )?;

                        ret.push(cell);
                        offset += 1;
                    }
                }

                Ok(ret)
            },
        )
    }
}
