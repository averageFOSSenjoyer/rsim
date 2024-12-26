use crate::component::Component;
use crate::event::Event;
use crate::send;
use crate::sim_manager::SimManager;
use crate::simple_component::simple_event::SimpleEvent;
use crate::task::Task;
use crate::types::Input;
use crate::types::Output;
use crate::types::{ComponentId, EventId};
use crate::{ack, enq};
use crossbeam_channel::{unbounded, Sender};
use rsim_macro::ComponentAttribute;
use std::sync::{Arc, Mutex};

#[ComponentAttribute({
"port": {
    "output": [
        ["output", "(u128,bool)"]
    ],
    "clock": true
}
})]
pub struct SimpleSender {
    num_packets: u128,
    sent_count: u128,
}

impl SimpleSender {
    pub fn new(
        component_id: ComponentId,
        sim_manager: Arc<SimManager>,
        num_packets: u128,
        output: Output,
        ack_sender: Sender<EventId>,
    ) -> Arc<Mutex<Self>> {
        let clock_tick_channel = unbounded();
        Arc::new(Mutex::new(SimpleSender {
            component_id,
            sim_manager,
            num_packets,
            output,
            sent_count: 0,
            clock_sender: clock_tick_channel.0,
            clock_receiver: clock_tick_channel.1,
            ack_sender,
        }))
    }
}

impl SimpleSender {
    fn init_impl(&mut self) {
        self.sim_manager.register_do_not_end(self.component_id);
    }

    fn reset_impl(&mut self) {
        self.sent_count = 0;
    }

    fn poll_impl(&mut self) {}

    fn on_clock(&mut self) {
        let recv_time = self.sim_manager.get_curr_cycle();

        if self.sent_count < self.num_packets {
            let is_last = self.sent_count == self.num_packets - 1;

            let event = SimpleEvent::new(
                recv_time + 10,
                self.sent_count,
                is_last,
                self.sim_manager.request_new_event_id(),
            );
            println!(
                "SimpleSender sent event: {:?} @ {:?} eid {}",
                self.sent_count,
                recv_time,
                event.get_event_id()
            );
            send!(self, self.output, event);
        } else {
            self.sim_manager.register_can_end(self.component_id);
        }

        self.sent_count += 1;
    }

    fn on_comb(&mut self) {}
}
