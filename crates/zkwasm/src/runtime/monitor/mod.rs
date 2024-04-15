use std::cell::RefCell;
use std::rc::Rc;

use wasmi::monitor::Monitor;
use wasmi::tracer::Observer;

pub mod plugins;

pub mod statistic_monitor;
pub mod table_monitor;

pub trait WasmiMonitor: Monitor {
    fn expose_observer(&self) -> Rc<RefCell<Observer>>;
}
