use crate::backend::util::event::WordEvent;
use crate::backend::util::types::Byte;
use crate::backend::util::types::Word;
use crate::send_word;
use crossbeam_channel::{unbounded, Sender};
use rsim_core::ack;
use rsim_core::component::Component;
use rsim_core::enq;
use rsim_core::event::get_inner;
use rsim_core::send;
use rsim_core::sim_manager::SimManager;
use rsim_core::task::Task;
use rsim_core::types::Input;
use rsim_core::types::{ComponentId, EventId, Output};
use rsim_macro::ComponentAttribute;
use std::fmt::{Debug, Formatter};
use std::sync::Arc;

#[ComponentAttribute({
"port": {
    "input": [
        ["load", "Byte"],
        ["mar", "Word"],
        ["rs2_data", "Word"]
    ],
    "output": [
        ["out", "Word"]
    ],
    "clock": true
}
})]
pub struct DataOut {
    pub data_inner: Word,
}

impl DataOut {
    pub fn new(
        component_id: ComponentId,
        sim_manager: Arc<SimManager>,
        ack_sender: Sender<EventId>,
        load_receiver: Input,
        mar_receiver: Input,
        rs2_data_receiver: Input,
        out: Output,
    ) -> Self {
        let clock_channel = unbounded();
        DataOut {
            data_inner: Default::default(),
            component_id,
            sim_manager,
            ack_sender,
            clock_sender: clock_channel.0,
            clock_receiver: clock_channel.1,
            load_receiver,
            load: Default::default(),
            load_old: Default::default(),
            mar_receiver,
            mar: Default::default(),
            mar_old: Default::default(),
            rs2_data_receiver,
            rs2_data: Default::default(),
            rs2_data_old: Default::default(),
            out,
        }
    }

    fn init_impl(&mut self) {}

    fn reset_impl(&mut self) {}

    fn poll_impl(&mut self) {}

    fn on_clock(&mut self) {
        if self.load.is_something_nonzero() {
            self.data_inner = self.rs2_data;
        }
    }

    fn on_comb(&mut self) {
        if let Some(mar) = Into::<Option<u32>>::into(self.mar) {
            // println!("{:?} {:02x}", self.data_inner << Word::from(8 * (mar & 0x3)), mar);
            send_word!(
                self,
                self.out,
                self.data_inner << Word::from(8 * (mar & 0x3))
            );
        }
    }
}

impl Debug for DataOut {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "DataOut: {{inner: {:?}, rs2_data: {:?}, mar: {:?}}}",
            self.data_inner, self.rs2_data, self.mar
        )
    }
}
