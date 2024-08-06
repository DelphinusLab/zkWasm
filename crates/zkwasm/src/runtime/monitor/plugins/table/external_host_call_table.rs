use specs::external_host_call_table::ExternalHostCallEntry;
use specs::external_host_call_table::ExternalHostCallTable as ExternalTable;

#[derive(Default)]
pub(super) struct ExternalHostCallTable {
    //    pub(super) current: Vec<ExternalHostCallEntry>,
    slices: Vec<ExternalTable>,
}

impl ExternalHostCallTable {
    // pub(super) fn push(&mut self, entry: ExternalHostCallEntry) {
    //     self.current.push(entry);
    // }

    // pub(super) fn flush(&mut self) {
    //     let slice = std::mem::replace(&mut self.current, Vec::new());
    //     let table = ExternalTable::new(slice);

    //     self.slices.push(table);
    // }

    pub(super) fn push_slice(&mut self, table: ExternalTable) {
        self.slices.push(table);
    }

    pub(super) fn finalized(mut self) -> Vec<ExternalTable> {
        //        self.flush();

        self.slices
    }
}
