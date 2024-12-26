use crate::event::Event;
use crossbeam_channel::{Receiver, Sender};

pub type ComponentId = u64;
pub type EventId = u128;
pub type Cycle = u128;
pub type Input = Receiver<Box<dyn Event>>;
pub type Output = Sender<Box<dyn Event>>;
