use std::path::PathBuf;

use halo2_proofs::arithmetic::MultiMillerLoop;
use threadpool::ThreadPool;
use wasmi::tracer::SliceDumper;
use wasmi::RuntimeValue;

use crate::loader::ZkWasmLoader;
use crate::runtime::ExecutionResult;

use super::slice::Slice;
use super::slice::Slices;

impl<E: MultiMillerLoop> ZkWasmLoader<E> {
<<<<<<< HEAD
=======
    pub(crate) fn compute_slice_capability(&self) -> usize {
        ((1 << self.k) - 200) / EVENT_TABLE_ENTRY_ROWS as usize
    }

>>>>>>> 1fd2066 (fix circuit_without_witness)
    pub fn slice(&self, execution_result: ExecutionResult<RuntimeValue>) -> Slices {
        Slices::new(
            execution_result.tables.unwrap(),
            self.compute_slice_capability(),
        )
    }
}

#[derive(Default)]
pub struct WitnessDumper {
    dump_enabled: bool,
    capacity: usize,
    output_dir: PathBuf,
    slice_index: u32,
    thread_pool: ThreadPool,
}

impl WitnessDumper {
    pub(crate) fn new(dump_enabled: bool, capacity: usize, output_dir: Option<PathBuf>) -> Self {
        let thread_pool = threadpool::Builder::new().build();
        let slice_index = 0;
        let output_dir = output_dir.unwrap_or_else(|| std::env::current_dir().unwrap());
        WitnessDumper {
            dump_enabled,
            capacity,
            output_dir,
            slice_index,
            thread_pool,
        }
    }
}

impl SliceDumper for WitnessDumper {
    fn dump(&mut self, tables: specs::Tables) {
        match self.dump_enabled {
            true => {
                cfg_if::cfg_if! {
                    if #[cfg(feature = "continuation")] {
                        let slice = Slice::new(tables, self.capacity);
                        let mut dir = self.output_dir.clone();
                        dir.push(self.slice_index.to_string());

                        while self.thread_pool.queued_count() > 0 {
                            std::thread::sleep(std::time::Duration::from_millis(10));
                        }

                        self.thread_pool.clone().execute(move || {
                            slice.write_flexbuffers(Some(dir));
                        });

                        self.slice_index += 1;
                    }
                }
            }
            false => {}
        }
    }

    fn dump_enabled(&self) -> bool {
        self.dump_enabled
    }

    fn get_capacity(&self) -> usize {
        self.capacity
    }
}
