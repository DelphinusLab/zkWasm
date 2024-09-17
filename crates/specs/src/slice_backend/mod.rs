use std::io;
use std::path::Path;

use serde::de::DeserializeOwned;
use serde::Deserialize;
use serde::Serialize;

use crate::etable::EventTable;
use crate::external_host_call_table::ExternalHostCallTable;
use crate::jtable::FrameTable;

#[derive(Serialize, Deserialize)]
pub struct Slice {
    pub etable: EventTable,
    pub frame_table: FrameTable,
    pub external_host_call_table: ExternalHostCallTable,
}

pub trait SliceBackend: Serialize + DeserializeOwned + Into<Slice> {
    fn write(
        &self,
        path_of_event_table: &Path,
        path_of_frame_table: &Path,
        path_of_external_host_call_table: &Path,
    ) -> io::Result<()>;
}

pub trait SliceBackendBuilder {
    type Output: SliceBackend;

    fn build(&mut self, slice: Slice) -> Self::Output;
}

pub type InMemoryBackendSlice = Slice;

impl SliceBackend for InMemoryBackendSlice {
    fn write(
        &self,
        path_of_event_table: &Path,
        path_of_frame_table: &Path,
        path_of_external_host_call_table: &Path,
    ) -> io::Result<()> {
        const DEBUG: bool = false;

        if DEBUG {
            self.etable.write(path_of_event_table)?;

            self.frame_table.write(path_of_frame_table)?;
        }

        self.external_host_call_table
            .write(path_of_external_host_call_table)?;

        Ok(())
    }
}

pub struct InMemoryBackendBuilder;

impl SliceBackendBuilder for InMemoryBackendBuilder {
    type Output = InMemoryBackendSlice;

    fn build(&mut self, slice: Slice) -> Self::Output {
        slice
    }
}
