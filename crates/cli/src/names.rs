#[inline(always)]
pub(crate) fn name_of_params(k: u32) -> String {
    format!("K{}.params", k)
}

#[inline(always)]
pub(crate) fn name_of_config(name: &str) -> String {
    format!("{}.zkwasm.config", name)
}

#[inline(always)]
pub(crate) fn name_of_circuit_data(name: &str) -> String {
    format!("{}.circuit.data", name)
}

// FIXME: adapt batcher crate, however the crate should provice this function
#[inline(always)]
pub(crate) fn name_of_loadinfo(name: &str) -> String {
    format!("{}.loadinfo.json", name)
}
