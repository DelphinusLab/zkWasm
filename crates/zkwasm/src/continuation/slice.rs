use specs::etable::EventTable;
use specs::CompilationTable;
use specs::ExecutionTable;
use specs::InitializationState;
use specs::Tables;

use crate::circuits::etable::EVENT_TABLE_ENTRY_ROWS;

pub struct Slice {
    table: Tables,
    current_slice: usize,
    total_slice: usize,
}

pub struct Slices {
    slices: Vec<Slice>,
    // the length of etable entries
    capability: usize,
}

impl Slices {
    /// Split table to slices
    pub fn from_table(table: Tables, capability: usize) -> Slices {
        let mut etable_slices = table
            .execution_tables
            .etable
            .entries()
            .chunks((capability - 1) * EVENT_TABLE_ENTRY_ROWS as usize)
            .collect::<Vec<_>>()
            .iter()
            .map(|v| v.to_vec())
            .collect::<Vec<Vec<_>>>();

        for index in 1..etable_slices.len() {
            let first_entry = etable_slices[index - 1].last().unwrap().clone();
            etable_slices[index].insert(0, first_entry);
        }

        let total_slice = etable_slices.len();
        let slices = etable_slices
            .into_iter()
            .enumerate()
            .map(|(current_slice, etable_slice)| Slice {
                table: Tables {
                    compilation_tables: CompilationTable {
                        itable: table.compilation_tables.itable.clone(),
                        // TODO: imtable should be updated.
                        imtable: table.compilation_tables.imtable.clone(),
                        elem_table: table.compilation_tables.elem_table.clone(),
                        configure_table: table.compilation_tables.configure_table,
                        static_jtable: table.compilation_tables.static_jtable.clone(),
                        // TODO: fid_of_entry should be updated or removed.
                        fid_of_entry: table.compilation_tables.fid_of_entry,
                    },
                    execution_tables: ExecutionTable {
                        initialization_state: if current_slice == 0 {
                            table.execution_tables.initialization_state.clone()
                        } else {
                            let first_etable_entry = etable_slice.first().unwrap();

                            InitializationState {
                                eid: first_etable_entry.eid,
                                fid: first_etable_entry.inst.fid,
                                iid: first_etable_entry.inst.iid,
                                frame_id: first_etable_entry.last_jump_eid,
                                sp: first_etable_entry.sp,
                                initial_memory_pages: first_etable_entry.allocated_memory_pages,
                                rest_jops: todo!(),
                                is_very_first_step: false,
                            }
                        },
                        etable: EventTable::new(etable_slice),
                        jtable: table.execution_tables.jtable.clone(),
                    },
                },
                current_slice,
                total_slice,
            })
            .collect();

        Slices { slices, capability }
    }
}
