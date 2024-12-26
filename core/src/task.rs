use crate::event::Event;
use crate::types::Output;
use std::cmp::Ordering;

#[derive(Debug)]
pub struct Task {
    pub event: Box<dyn Event>,
    pub event_callback: Output,
}

impl Task {
    pub fn new(event: Box<dyn Event>, event_callback: Output) -> Task {
        Task {
            event,
            event_callback,
        }
    }
}

impl PartialEq for Task {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self.event.as_ref(), other.event.as_ref())
    }
}

impl Eq for Task {}

impl PartialOrd<Self> for Task {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// While the crossbeam channels are FIFOs with stable ordering,
/// rust's binary heap does not.
///
/// This means while events from the component to the sim manager will arrive with the same order,
/// the event q may not order them in arrival order.
///
/// This is combated using an event_id, originally for the rob.
/// Since the event_id always increases from the perspective of a single component,
/// later events will overwrite earlier events.
impl Ord for Task {
    fn cmp(&self, other: &Self) -> Ordering {
        if self.event.get_scheduled_time() == other.event.get_scheduled_time() {
            other.event.get_event_id().cmp(&self.event.get_event_id())
        } else {
            other
                .event
                .get_scheduled_time()
                .cmp(&self.event.get_scheduled_time())
        }
    }
}
