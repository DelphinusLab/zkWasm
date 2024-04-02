//use instruction_statistic::InstructionStatistic;
use specs::Tables;

mod helper;
// mod instruction_statistic;

pub trait Profiler {
    fn profile_tables(&self);
}

impl Profiler for Tables {
    fn profile_tables(&self) {
        //self.profile_instruction();
    }
}
