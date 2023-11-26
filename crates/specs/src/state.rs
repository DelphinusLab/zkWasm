use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
pub struct InitializationState<T> {
    pub eid: T,
    pub fid: T,
    pub iid: T,
    pub frame_id: T,
    pub sp: T,

    pub host_public_inputs: T,
    pub context_in_index: T,
    pub context_out_index: T,
    pub external_host_call_call_index: T,

    pub initial_memory_pages: T,
    pub maximal_memory_pages: T,

    #[cfg(feature = "continuation")]
    pub jops: T,
}

impl<T> InitializationState<T> {
    pub fn field_count() -> usize {
        if cfg!(feature = "continuation") {
            12
        } else {
            11
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
            external_host_call_call_index: Default::default(),

            initial_memory_pages: Default::default(),
            maximal_memory_pages: Default::default(),

            #[cfg(feature = "continuation")]
            jops: Default::default(),
        }
    }
}

impl<T: Clone> InitializationState<T> {
    pub fn plain(&self) -> Vec<T> {
        let mut v = vec![];

        v.push(self.eid.clone());
        v.push(self.fid.clone());
        v.push(self.iid.clone());
        v.push(self.frame_id.clone());
        v.push(self.sp.clone());

        v.push(self.host_public_inputs.clone());
        v.push(self.context_in_index.clone());
        v.push(self.context_out_index.clone());
        v.push(self.external_host_call_call_index.clone());

        v.push(self.initial_memory_pages.clone());
        v.push(self.maximal_memory_pages.clone());

        #[cfg(feature = "continuation")]
        v.push(self.jops.clone());

        v
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
            external_host_call_call_index: f(&self.external_host_call_call_index),

            initial_memory_pages: f(&self.initial_memory_pages),
            maximal_memory_pages: f(&self.maximal_memory_pages),

            #[cfg(feature = "continuation")]
            jops: f(&self.jops),
        }
    }
}
