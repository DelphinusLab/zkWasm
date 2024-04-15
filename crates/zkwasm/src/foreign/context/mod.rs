use std::fs::File;
use std::io;
use std::io::Write;

pub mod circuits;
pub mod etable_op_configure;
pub mod runtime;

enum Op {
    ReadContext = 0,
    WriteContext = 1,
}

#[derive(Clone, Default)]
pub struct ContextOutput(pub Vec<u64>);

impl ContextOutput {
    pub fn write(&self, fd: &mut File) -> io::Result<()> {
        fd.write_all("0x".as_bytes())?;

        for value in &self.0 {
            let bytes = value.to_le_bytes();
            let s = hex::encode(bytes);
            fd.write_all(&s.as_bytes())?;
        }

        fd.write_all(":bytes-packed".as_bytes())?;

        Ok(())
    }
}
