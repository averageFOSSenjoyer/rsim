use crate::ack;
use crate::component::Component;
use crate::event::get_inner;
use crate::sim_manager::SimManager;
use crate::types::ComponentId;
use crate::types::EventId;
use crate::types::Input;
use crossbeam_channel::Sender;
use rsim_macro::ComponentAttribute;
use std::sync::{Arc, Mutex};

#[ComponentAttribute({
"port": {
    "input": [
        ["input", "(u128,bool)"]
    ]
}
})]
pub struct SimpleReceiver {}

impl SimpleReceiver {
    pub fn new(
        component_id: ComponentId,
        sim_manager: Arc<SimManager>,
        input_receiver: Input,
        ack_sender: Sender<u128>,
    ) -> Arc<Mutex<Self>> {
        Arc::new(Mutex::new(SimpleReceiver {
            component_id,
            sim_manager,
            input_receiver,
            ack_sender,
            input: (0, false),
            input_old: (0, false),
        }))
    }
}

impl SimpleReceiver {
    fn init_impl(&mut self) {
        self.sim_manager.register_do_not_end(self.component_id);
    }

    fn reset_impl(&mut self) {
        self.input = (0, false);
    }

    fn poll_impl(&mut self) {}

    fn on_comb(&mut self) {
        // println!(
        //     "SimpleReceiver received event: {:?} @ {:?}",
        //     self.input.0, self.input.1
        // );
        if self.input.1 {
            self.sim_manager.register_can_end(self.component_id);
        }
    }
}
