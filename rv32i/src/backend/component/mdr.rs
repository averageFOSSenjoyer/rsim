use crate::backend::util::event::WordEvent;
use crate::backend::util::types::Byte;
use crate::backend::util::types::Word;
use crate::send_word;
use crossbeam_channel::unbounded;
use crossbeam_channel::Sender;
use rsim_core::ack;
use rsim_core::component::Component;
use rsim_core::enq;
use rsim_core::event::get_inner;
use rsim_core::send;
use rsim_core::sim_manager::SimManager;
use rsim_core::task::Task;
use rsim_core::types::ComponentId;
use rsim_core::types::EventId;
use rsim_core::types::Input;
use rsim_core::types::Output;
use rsim_macro::ComponentAttribute;
use std::fmt::{Debug, Formatter};
use std::sync::Arc;

#[ComponentAttribute({
"port": {
    "input": [
        ["load", "Byte"],
        ["data", "Word"]
    ],
    "output": [
        ["out_ir", "Word"],
        ["out_regfile_mux", "Word"]
    ],
    "clock": true
}
})]
pub struct Mdr {
    data_inner: Word,
}

impl Mdr {
    pub fn new(
        component_id: ComponentId,
        sim_manager: Arc<SimManager>,
        ack_sender: Sender<EventId>,
        load_receiver: Input,
        data_receiver: Input,
        out_ir: Output,
        out_regfile_mux: Output,
    ) -> Self {
        let clock_channel = unbounded();
        Mdr {
            data_inner: Default::default(),
            component_id,
            sim_manager,
            ack_sender,
            clock_sender: clock_channel.0,
            clock_receiver: clock_channel.1,
            load_receiver,
            load: Default::default(),
            load_old: Default::default(),
            data_receiver,
            data: Default::default(),
            data_old: Default::default(),
            out_ir,
            out_regfile_mux,
        }
    }
    fn init_impl(&mut self) {}

    fn reset_impl(&mut self) {}

    fn poll_impl(&mut self) {}

    fn on_clock(&mut self) {
        if self.load.is_something_nonzero() {
            self.data_inner = self.data;
        }
    }

    fn on_comb(&mut self) {
        send_word!(self, self.out_ir, self.data_inner);
        send_word!(self, self.out_regfile_mux, self.data_inner);
    }
}

impl Debug for Mdr {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "MDR: {{data: {:?}, data_inner: {:?}, load: {:?}}}",
            self.data, self.data_inner, self.load
        )
    }
}
