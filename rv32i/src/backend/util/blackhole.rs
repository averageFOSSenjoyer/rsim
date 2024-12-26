#![allow(dead_code)]
use crate::backend::util::types::Word;
use crossbeam_channel::Sender;
use rsim_core::ack;
use rsim_core::component::Component;
use rsim_core::event::get_inner;
use rsim_core::sim_manager::SimManager;
use rsim_core::types::EventId;
use rsim_core::types::{ComponentId, Input};
use rsim_macro::ComponentAttribute;
use std::sync::{Arc, Mutex};

#[ComponentAttribute({
"port": {
    "input": [
        ["input", "Word"]
    ],
    "clock": false
}
})]
pub struct WordBlackhole {}

impl WordBlackhole {
    pub fn new(
        component_id: ComponentId,
        sim_manager: Arc<SimManager>,
        input_receiver: Input,
        ack_sender: Sender<u128>,
    ) -> Arc<Mutex<Self>> {
        Arc::new(Mutex::new(WordBlackhole {
            component_id,
            sim_manager,
            input_receiver,
            ack_sender,
            input: Word::unknown(),
            input_old: Default::default(),
        }))
    }
}

impl WordBlackhole {
    fn init_impl(&mut self) {}

    fn reset_impl(&mut self) {
        self.input = Word::unknown();
    }

    fn poll_impl(&mut self) {}

    fn on_comb(&mut self) {}

    pub fn get_input(&self) -> Word {
        self.input
    }
}
