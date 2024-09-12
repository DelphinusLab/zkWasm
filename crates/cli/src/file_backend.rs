use std::collections::VecDeque;
use std::path::PathBuf;

use specs::etable::EventTable;
use specs::external_host_call_table::ExternalHostCallTable;
use specs::jtable::FrameTable;
use specs::slice_backend::Slice;
use specs::slice_backend::SliceBackend;

use crate::names::name_of_etable_slice;
use crate::names::name_of_external_host_call_table_slice;
use crate::names::name_of_frame_table_slice;

struct SlicePath {
    event_table: PathBuf,
    frame_table: PathBuf,
    external_host_call_table: PathBuf,
}

impl Into<Slice> for &SlicePath {
    fn into(self) -> Slice {
        Slice {
            etable: EventTable::read(&self.event_table).unwrap(),
            frame_table: FrameTable::read(&self.frame_table).unwrap(),
            external_host_call_table: ExternalHostCallTable::read(&self.external_host_call_table)
                .unwrap(),
        }
    }
}

pub(crate) struct FileBackend {
    peeked: Option<Slice>,

    dir_path: PathBuf,
    name: String,
    slices: VecDeque<SlicePath>,
}

impl FileBackend {
    pub(crate) fn new(name: String, dir_path: PathBuf) -> Self {
        FileBackend {
            peeked: None,

            dir_path,
            name,
            slices: VecDeque::new(),
        }
    }
}

impl SliceBackend for FileBackend {
    fn push(&mut self, slice: Slice) {
        let index = self.slices.len();

        let event_table = {
            let path = self
                .dir_path
                .join(PathBuf::from(name_of_etable_slice(&self.name, index)));
            slice.etable.write(&path).unwrap();
            path
        };

        let frame_table = {
            let path = self
                .dir_path
                .join(PathBuf::from(name_of_frame_table_slice(&self.name, index)));
            slice.frame_table.write(&path).unwrap();
            path
        };

        let external_host_call_table = {
            let path = self
                .dir_path
                .join(PathBuf::from(name_of_external_host_call_table_slice(
                    &self.name, index,
                )));
            slice.external_host_call_table.write(&path).unwrap();
            path
        };

        self.slices.push_back(SlicePath {
            event_table,
            frame_table,
            external_host_call_table,
        });
    }

    fn pop(&mut self) -> Option<Slice> {
        match self.peeked.take() {
            Some(v) => Some(v),
            None => self.slices.pop_front().map(|slice| (&slice).into()),
        }
    }

    fn first(&mut self) -> Option<&Slice> {
        if self.peeked.is_none() {
            self.peeked = self.slices.pop_front().map(|slice| (&slice).into());
        }

        self.peeked.as_ref()
    }

    fn len(&self) -> usize {
        self.slices.len() + self.peeked.is_some() as usize
    }

    fn is_empty(&self) -> bool {
        self.slices.is_empty() && self.peeked.is_none()
    }

    fn for_each<'a>(&'a self, f: Box<dyn Fn((usize, &Slice)) + 'a>) {
        let mut offset = 0usize;

        if let Some(slice) = self.peeked.as_ref() {
            f((offset, slice));
            offset = offset + 1;
        }

        self.slices.iter().enumerate().for_each(|(index, slice)| {
            let slice: Slice = slice.into();
            f((index + offset, &slice))
        })
    }
}
