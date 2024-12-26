use crate::backend::util::event::WordEvent;
use crate::backend::util::types::Byte;
use crate::backend::util::types::{mux_sel, Word};
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
        ["out_control", "Word"],
        ["out_regfile_mux", "Word"],
        ["out_data_out", "Word"],
        ["out_mem_ctl", "Word"]
    ],
    "clock": true
}
})]
pub struct Mar {
    pub data_inner: Word,
}

impl Mar {
    pub fn new(
        component_id: ComponentId,
        sim_manager: Arc<SimManager>,
        ack_sender: Sender<EventId>,
        load_receiver: Input,
        data_receiver: Input,
        out_control: Output,
        out_regfile_mux: Output,
        out_data_out: Output,
        out_mem_ctl: Output,
    ) -> Self {
        let clock_channel = unbounded();
        Mar {
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
            out_control,
            out_regfile_mux,
            out_data_out,
            out_mem_ctl,
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
        send_word!(self, self.out_control, self.data_inner);
        send_word!(self, self.out_regfile_mux, self.data_inner);
        send_word!(self, self.out_data_out, self.data_inner);
        send_word!(
            self,
            self.out_mem_ctl,
            self.data_inner & Word::from(0xFFFFFFFCu32)
        );
    }
}

impl Debug for Mar {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "MAR: {{data_inner: {:?}, load: {:?}}}",
            self.data_inner, self.load
        )
    }
}

#[ComponentAttribute({
"port": {
    "input": [
        ["pc", "Word"],
        ["alu_out", "Word"],
        ["sel", "Byte"]
    ],
    "output": [
        ["out", "Word"]
    ]
}
})]
pub struct MarMux {}

impl MarMux {
    pub fn new(
        component_id: ComponentId,
        sim_manager: Arc<SimManager>,
        ack_sender: Sender<EventId>,
        pc_receiver: Input,
        alu_out_receiver: Input,
        sel_receiver: Input,
        out: Output,
    ) -> Self {
        MarMux {
            component_id,
            sim_manager,
            ack_sender,
            pc_receiver,
            pc: Default::default(),
            pc_old: Default::default(),
            alu_out_receiver,
            alu_out: Default::default(),
            alu_out_old: Default::default(),
            sel_receiver,
            sel: Default::default(),
            sel_old: Default::default(),
            out,
        }
    }

    fn init_impl(&mut self) {}

    fn reset_impl(&mut self) {}

    fn poll_impl(&mut self) {}

    fn on_comb(&mut self) {
        let out = match self.sel.into() {
            Some(mux_sel::mar::PC_OUT) => self.pc,
            Some(mux_sel::mar::ALU_OUT) => self.alu_out,
            _ => Word::unknown(),
        };

        send_word!(self, self.out, out);
    }
}

impl Debug for MarMux {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "MARMUX: {{pc: {:?}, alu_out: {:?}, sel: {:?}}}",
            self.pc, self.alu_out, self.sel
        )
    }
}
