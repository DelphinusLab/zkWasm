use std::fs;
use std::io;
use std::path::Path;
use std::path::PathBuf;

use serde::Deserialize;
use serde::Serialize;
use specs::etable::EventTable;
use specs::external_host_call_table::ExternalHostCallTable;
use specs::jtable::FrameTable;
use specs::slice_backend::Slice;
use specs::slice_backend::SliceBackend;
use specs::slice_backend::SliceBackendBuilder;

use crate::names::name_of_etable_slice;
use crate::names::name_of_external_host_call_table_slice;
use crate::names::name_of_frame_table_slice;

struct SlicePath {
    event_table: PathBuf,
    frame_table: PathBuf,
    external_host_call_table: PathBuf,
}

impl From<&SlicePath> for Slice {
    fn from(val: &SlicePath) -> Self {
        Slice {
            etable: EventTable::read(&val.event_table).unwrap(),
            frame_table: FrameTable::read(&val.frame_table).unwrap(),
            external_host_call_table: ExternalHostCallTable::read(&val.external_host_call_table)
                .unwrap(),
        }
    }
}

#[derive(Serialize, Deserialize)]
pub(crate) struct FileBackendSlice {
    event_table: PathBuf,
    frame_table: PathBuf,
    external_host_call_table: PathBuf,
}

impl From<FileBackendSlice> for Slice {
    fn from(value: FileBackendSlice) -> Self {
        Slice {
            etable: EventTable::read(&value.event_table).unwrap(),
            frame_table: FrameTable::read(&value.frame_table).unwrap(),
            external_host_call_table: ExternalHostCallTable::read(&value.external_host_call_table)
                .unwrap(),
        }
    }
}

impl SliceBackend for FileBackendSlice {
    fn write(
        &self,
        path_of_event_table: &Path,
        path_of_frame_table: &Path,
        path_of_external_host_call_table: &Path,
    ) -> io::Result<()> {
        if self.event_table.as_path().canonicalize()? != path_of_event_table.canonicalize()? {
            fs::copy(self.event_table.as_path(), path_of_event_table)?;
        }
        if self.frame_table.as_path().canonicalize()? != path_of_frame_table.canonicalize()? {
            fs::copy(self.frame_table.as_path(), path_of_frame_table)?;
        }
        if self.external_host_call_table.as_path().canonicalize()?
            != path_of_external_host_call_table.canonicalize()?
        {
            fs::copy(
                self.external_host_call_table.as_path(),
                path_of_external_host_call_table,
            )?;
        }

        Ok(())
    }
}

pub(crate) struct FileBackendBuilder {
    name: String,
    dir: PathBuf,
    index: usize,
}

impl FileBackendBuilder {
    pub(crate) fn new(name: String, dir: PathBuf) -> Self {
        Self {
            name,
            dir,
            index: 0,
        }
    }
}

impl SliceBackendBuilder for FileBackendBuilder {
    type Output = FileBackendSlice;

    fn build(&mut self, slice: Slice) -> Self::Output {
        let event_table = {
            let path = self
                .dir
                .join(PathBuf::from(name_of_etable_slice(&self.name, self.index)));
            slice.etable.write(&path).unwrap();
            path
        };

        let frame_table = {
            let path = self.dir.join(PathBuf::from(name_of_frame_table_slice(
                &self.name, self.index,
            )));
            slice.frame_table.write(&path).unwrap();
            path
        };

        let external_host_call_table = {
            let path = self
                .dir
                .join(PathBuf::from(name_of_external_host_call_table_slice(
                    &self.name, self.index,
                )));
            slice.external_host_call_table.write(&path).unwrap();
            path
        };

        self.index += 1;

        FileBackendSlice {
            event_table,
            frame_table,
            external_host_call_table,
        }
    }
}
