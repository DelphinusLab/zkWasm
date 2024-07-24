use std::fs::File;
use std::io;
use std::io::Write;

use specs::host_function::HostPlugin;
use specs::step::StepInfo;

pub mod circuits;
pub mod etable_op_configure;
pub mod runtime;

enum Op {
    ReadContext = 0,
    WriteContext = 1,
}

pub struct ContextOutput(pub Vec<u64>);

pub fn try_get_context_input_from_step_info(step_info: &StepInfo) -> Option<u64> {
    match step_info {
        StepInfo::CallHost {
            plugin: HostPlugin::Context,
            op_index_in_plugin,
            ret_val,
            ..
        } => {
            if *op_index_in_plugin == Op::ReadContext as usize {
                Some(ret_val.unwrap())
            } else {
                None
            }
        }
        _ => None,
    }
}

pub fn try_get_context_output_from_step_info(step_info: &StepInfo) -> Option<u64> {
    match step_info {
        StepInfo::CallHost {
            plugin: HostPlugin::Context,
            op_index_in_plugin,
            args,
            ..
        } => {
            if *op_index_in_plugin == Op::WriteContext as usize {
                Some(args[0])
            } else {
                None
            }
        }
        _ => None,
    }
}

impl ContextOutput {
    pub fn write(&self, fd: &mut File) -> io::Result<()> {
        fd.write_all("0x".as_bytes())?;

        for value in &self.0 {
            let bytes = value.to_le_bytes();
            let s = hex::encode(bytes);
            fd.write_all(s.as_bytes())?;
        }

        fd.write_all(":bytes-packed".as_bytes())?;

        Ok(())
    }
}
