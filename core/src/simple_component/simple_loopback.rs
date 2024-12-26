use crate::component::Component;
use crate::event::get_inner;
use crate::send;
use crate::sim_manager::SimManager;
use crate::simple_component::simple_event::SimpleEvent;
use crate::task::Task;
use crate::types::ComponentId;
use crate::types::Input;
use crate::types::{EventId, Output};
use crate::{ack, enq};
use crossbeam_channel::{unbounded, Sender};
use rsim_macro::ComponentAttribute;
use std::sync::{Arc, Mutex};

#[ComponentAttribute({
"port": {
    "input": [
        ["input", "(u128,bool)"]
    ],
    "output": [
        ["output", "(u128,bool)"]
    ],
    "clock": true
}
})]
pub struct SimpleLoopback {
    num_packets: u128,
    sent_count: u128,
}

impl SimpleLoopback {
    pub fn new(
        component_id: ComponentId,
        sim_manager: Arc<SimManager>,
        num_packets: u128,
        input_receiver: Input,
        output: Output,
        ack_sender: Sender<EventId>,
    ) -> Arc<Mutex<Self>> {
        let clock_tick_channel = unbounded();
        Arc::new(Mutex::new(SimpleLoopback {
            component_id,
            sim_manager,
            num_packets,
            input_receiver,
            input: (0, false),
            input_old: (0, false),
            output,
            sent_count: 0,
            clock_sender: clock_tick_channel.0,
            clock_receiver: clock_tick_channel.1,
            ack_sender,
        }))
    }

    fn init_impl(&mut self) {
        self.sim_manager.register_do_not_end(self.component_id);
    }

    fn reset_impl(&mut self) {
        self.input = (0, false);
        self.sent_count = 0;
    }

    fn poll_impl(&mut self) {}

    fn on_clock(&mut self) {
        let recv_time = self.sim_manager.get_curr_cycle();

        if self.sent_count < self.num_packets {
            let is_last = self.sent_count == self.num_packets - 1;
            let event = SimpleEvent::new(
                recv_time + 1,
                self.sent_count,
                is_last,
                self.sim_manager.request_new_event_id(),
            );
            send!(self, self.output, event);
            println!(
                "SimpleLoopback sent event: {:?} @ {:?}",
                self.sent_count, recv_time
            );
        } else {
            self.sim_manager.register_can_end(self.component_id);
        }

        self.sent_count += 1;
    }

    fn on_comb(&mut self) {
        println!(
            "SimpleLoopback received {} @ {}",
            self.input.0,
            self.sim_manager.get_curr_cycle()
        );
    }
}
