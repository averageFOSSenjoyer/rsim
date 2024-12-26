use crate::types::{Cycle, EventId};
use std::any::Any;
use std::fmt::Debug;
use std::ops::Deref;

pub trait Event: Send + Sync + Debug {
    fn is_clock_event(&self) -> bool {
        false
    }
    fn get_event_id(&self) -> EventId;
    fn get_scheduled_time(&self) -> Cycle;
    fn set_scheduled_time(&mut self, scheduled_time: Cycle);
    fn get_inner(&self) -> Box<dyn Any>;
}

pub fn get_inner<T: Copy + 'static>(event: &dyn Event) -> T {
    *(event.get_inner().downcast::<T>().unwrap().deref())
}
