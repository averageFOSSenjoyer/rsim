use crate::component::Component;
use crate::event::get_inner;
use crate::send;
use crate::sim_manager::SimManager;
use crate::simple_component::simple_event::SimpleEvent;
use crate::task::Task;
use crate::types::ComponentId;
use crate::types::EventId;
use crate::types::Input;
use crate::types::Output;
use crate::{ack, enq};
use crossbeam_channel::Sender;
use rsim_macro::ComponentAttribute;
use std::sync::{Arc, Mutex};

#[ComponentAttribute({
"port": {
    "input": [
        ["input", "(u128,bool)"]
    ],
    "output": [
        ["output", "(u128,bool)"]
    ]
}
})]
pub struct SimpleLink {}

impl SimpleLink {
    pub fn new(
        component_id: ComponentId,
        sim_manager: Arc<SimManager>,
        input_receiver: Input,
        output: Output,
        ack_sender: Sender<u128>,
    ) -> Arc<Mutex<Self>> {
        Arc::new(Mutex::new(SimpleLink {
            component_id,
            sim_manager,
            input_receiver,
            output,
            ack_sender,
            input: (0, false),
            input_old: (0, false),
        }))
    }
}

impl SimpleLink {
    fn init_impl(&mut self) {}

    fn reset_impl(&mut self) {
        self.input = (0, false);
    }

    fn poll_impl(&mut self) {}

    fn on_comb(&mut self) {
        let event = SimpleEvent::new(
            self.sim_manager.get_curr_cycle(),
            self.input.0,
            self.input.1,
            self.sim_manager.request_new_event_id(),
        );
        send!(self, self.output, event);
    }
}
