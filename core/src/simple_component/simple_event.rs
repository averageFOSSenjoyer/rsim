use crate::event::Event;
use crate::types::{Cycle, EventId};
use std::any::Any;

#[derive(Debug)]
pub struct SimpleEvent {
    scheduled_time: Cycle,
    event_id: EventId,
    packet_id: u128,
    is_last: bool,
}

impl SimpleEvent {
    pub fn new(scheduled_time: Cycle, packet_id: u128, is_last: bool, event_id: EventId) -> Self {
        SimpleEvent {
            scheduled_time,
            event_id,
            packet_id,
            is_last,
        }
    }
}

impl Event for SimpleEvent {
    fn get_event_id(&self) -> EventId {
        self.event_id
    }

    fn get_scheduled_time(&self) -> Cycle {
        self.scheduled_time
    }

    fn set_scheduled_time(&mut self, scheduled_time: Cycle) {
        self.scheduled_time = scheduled_time
    }

    fn get_inner(&self) -> Box<dyn Any> {
        Box::new((self.packet_id, self.is_last))
    }
}
