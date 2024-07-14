use serde::Deserialize;
use serde::Serialize;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InitializationState<T> {
    pub eid: T,
    pub fid: T,
    pub iid: T,
    pub frame_id: T,
    pub sp: T,

    pub host_public_inputs: T,
    pub context_in_index: T,
    pub context_out_index: T,

    pub initial_memory_pages: T,
    pub maximal_memory_pages: T,
}

pub const INITIALIZATION_STATE_FIELD_COUNT: usize = 10;
impl<T> InitializationState<T> {
    // TODO: try to remove the magic number
    pub fn field_count() -> usize {
        INITIALIZATION_STATE_FIELD_COUNT
    }

    pub fn zip_for_each<U, E>(
        &self,
        other: &Self,
        mut closure: impl FnMut(&T, &T) -> Result<U, E>,
    ) -> Result<(), E> {
        closure(&self.eid, &other.eid)?;
        closure(&self.fid, &other.fid)?;
        closure(&self.iid, &other.iid)?;
        closure(&self.frame_id, &other.frame_id)?;
        closure(&self.sp, &other.sp)?;

        closure(&self.host_public_inputs, &other.host_public_inputs)?;
        closure(&self.context_in_index, &other.context_in_index)?;
        closure(&self.context_out_index, &other.context_out_index)?;

        closure(&self.initial_memory_pages, &other.initial_memory_pages)?;
        closure(&self.maximal_memory_pages, &other.maximal_memory_pages)?;

        Ok(())
    }

    pub fn for_each<U>(&self, f: impl FnMut(&T) -> U) {
        self.map(f);
    }

    pub fn map<U>(&self, mut f: impl FnMut(&T) -> U) -> InitializationState<U> {
        InitializationState {
            eid: f(&self.eid),
            fid: f(&self.fid),
            iid: f(&self.iid),
            frame_id: f(&self.frame_id),
            sp: f(&self.sp),

            host_public_inputs: f(&self.host_public_inputs),
            context_in_index: f(&self.context_in_index),
            context_out_index: f(&self.context_out_index),

            initial_memory_pages: f(&self.initial_memory_pages),
            maximal_memory_pages: f(&self.maximal_memory_pages),
        }
    }
}

impl Default for InitializationState<u32> {
    fn default() -> Self {
        Self {
            eid: Default::default(),
            fid: Default::default(),
            iid: Default::default(),
            frame_id: Default::default(),
            sp: Default::default(),

            host_public_inputs: Default::default(),
            context_in_index: Default::default(),
            context_out_index: Default::default(),

            initial_memory_pages: Default::default(),
            maximal_memory_pages: Default::default(),
        }
    }
}

impl<T: Clone> InitializationState<T> {
    pub fn plain(&self) -> Vec<T> {
        vec![
            self.eid.clone(),
            self.fid.clone(),
            self.iid.clone(),
            self.frame_id.clone(),
            self.sp.clone(),
            self.host_public_inputs.clone(),
            self.context_in_index.clone(),
            self.context_out_index.clone(),
            self.initial_memory_pages.clone(),
            self.maximal_memory_pages.clone(),
        ]
    }
}

impl<T, E> InitializationState<Result<T, E>> {
    pub fn transpose(self) -> Result<InitializationState<T>, E> {
        Ok(InitializationState {
            eid: self.eid?,
            fid: self.fid?,
            iid: self.iid?,
            frame_id: self.frame_id?,
            sp: self.sp?,
            host_public_inputs: self.host_public_inputs?,
            context_in_index: self.context_in_index?,
            context_out_index: self.context_out_index?,
            initial_memory_pages: self.initial_memory_pages?,
            maximal_memory_pages: self.maximal_memory_pages?,
        })
    }
}
