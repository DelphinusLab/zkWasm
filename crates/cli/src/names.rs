#[inline(always)]
pub(crate) fn name_of_params(k: u32) -> String {
    format!("K{}.params", k)
}

#[inline(always)]
pub(crate) fn name_of_config(name: &str) -> String {
    format!("{}.zkwasm.config", name)
}

#[inline(always)]
pub(crate) fn name_of_circuit_data(name: &str, is_last_circuit: bool) -> String {
    if is_last_circuit {
        format!("{}.circuit.finalized.data", name)
    } else {
        format!("{}.circuit.ongoing.data", name)
    }
}

// FIXME: adapt batcher crate, however the crate should provice this function
#[inline(always)]
pub(crate) fn name_of_loadinfo(name: &str) -> String {
    format!("{}.loadinfo.json", name)
}

#[inline(always)]
pub(crate) fn name_of_witness(name: &str, index: usize) -> String {
    format!("{}.{}.witness.json", name, index)
}

#[inline(always)]
pub(crate) fn name_of_instance(name: &str, index: usize) -> String {
    format!("{}.{}.instance.data", name, index)
}

#[inline(always)]
pub(crate) fn name_of_transcript(name: &str, index: usize) -> String {
    format!("{}.{}.transcript.data", name, index)
}

#[inline(always)]
pub(crate) fn name_of_etable_slice(name: &str, index: usize) -> String {
    format!("{}.etable.{}.data", name, index)
}

#[inline(always)]
pub(crate) fn name_of_frame_table_slice(name: &str, index: usize) -> String {
    format!("{}.frame_table.{}.data", name, index)
}

#[inline(always)]
pub(crate) fn name_of_external_host_call_table_slice(_name: &str, index: usize) -> String {
    format!("external_host_table.{}.json", index)
}
