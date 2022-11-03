use super::{etable_compact::ETABLE_STEP_SIZE, mtable_compact::configure::STEP_SIZE};

pub const K: u32 = 20;
pub const VAR_COLUMNS: usize = 16;
const _MAX_ETABLE_ROWS: usize = 1usize << (K - 2);
pub const MAX_ETABLE_ROWS: usize = _MAX_ETABLE_ROWS / ETABLE_STEP_SIZE * ETABLE_STEP_SIZE;
const _MAX_MATBLE_ROWS: usize = 1usize << (K - 1);
pub const MAX_MATBLE_ROWS: usize = _MAX_MATBLE_ROWS / STEP_SIZE as usize * STEP_SIZE as usize;
pub const MAX_JATBLE_ROWS: usize = 1usize << (K - 6);
pub const IMTABLE_COLOMNS: usize = 2;

pub const POW_TABLE_LIMIT: u64 = 128;

pub const ETABLE_START_OFFSET: usize = 0;
pub const ETABLE_END_OFFSET: usize = ETABLE_START_OFFSET + MAX_ETABLE_ROWS;
pub const MTABLE_START_OFFSET: usize = 1usize << (K - 2);
pub const MTABLE_END_OFFSET: usize = MTABLE_START_OFFSET + MAX_MATBLE_ROWS;
pub const JTABLE_START_OFFSET: usize = (1usize << (K - 2)) * 3;
pub const JTABLE_END_OFFSET: usize = JTABLE_START_OFFSET + MAX_JATBLE_ROWS;
