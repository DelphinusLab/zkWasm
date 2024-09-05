use std::collections::VecDeque;

use super::Slice;
use super::SliceBackend;

#[derive(Default)]
pub struct InMemoryBackend {
    slices: VecDeque<Slice>,
}

impl Iterator for InMemoryBackend {
    type Item = Slice;

    fn next(&mut self) -> Option<Self::Item> {
        self.slices.pop_front()
    }
}

impl SliceBackend for InMemoryBackend {
    fn push(&mut self, slice: Slice) {
        self.slices.push_back(slice)
    }

    fn len(&self) -> usize {
        self.slices.len()
    }

    fn is_empty(&self) -> bool {
        self.slices.is_empty()
    }

    fn for_each1<'a>(&'a self, f: Box<dyn Fn((usize, &Slice)) + 'a>) {
        self.slices.iter().enumerate().for_each(f)
    }
}
