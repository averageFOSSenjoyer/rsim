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
use std::process::exit;
use std::sync::Arc;

#[ComponentAttribute({
"port": {
    "input": [
        ["load", "Byte"],
        ["data", "Word"]
    ],
    "output": [
        ["out_alu_mux1", "Word"],
        ["out_pc_mux", "Word"],
        ["out_mar_mux", "Word"],
        ["out_regfile_mux", "Word"]
    ],
    "clock": true
}
})]
pub struct Pc {
    pub data_inner: Word,
}

impl Pc {
    pub fn new(
        component_id: ComponentId,
        sim_manager: Arc<SimManager>,
        ack_sender: Sender<EventId>,
        load_receiver: Input,
        data_receiver: Input,
        out_alu_mux1: Output,
        out_pc_mux: Output,
        out_mar_mux: Output,
        out_regfile_mux: Output,
    ) -> Self {
        let clock_channel = unbounded();
        Pc {
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
            out_alu_mux1,
            out_pc_mux,
            out_mar_mux,
            out_regfile_mux,
        }
    }
    fn init_impl(&mut self) {
        self.data_inner = Word::from(0x40000000u32);
    }

    fn reset_impl(&mut self) {
        self.data_inner = Word::from(0x40000000u32);
    }

    fn poll_impl(&mut self) {}

    fn on_clock(&mut self) {
        if self.load.is_something_nonzero() {
            self.data_inner = self.data;
        }
    }

    fn on_comb(&mut self) {
        send_word!(self, self.out_alu_mux1, self.data_inner);
        send_word!(self, self.out_pc_mux, self.data_inner);
        send_word!(self, self.out_mar_mux, self.data_inner);
        send_word!(self, self.out_regfile_mux, self.data_inner);
    }
}

impl Debug for Pc {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if self.data_inner.has_unknown() {
            exit(0);
        }
        write!(f, "PC {{{:?}}}", self.data_inner)
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
pub struct PcMux {}

impl PcMux {
    pub fn new(
        component_id: ComponentId,
        sim_manager: Arc<SimManager>,
        ack_sender: Sender<EventId>,
        pc_receiver: Input,
        alu_out_receiver: Input,
        sel_receiver: Input,
        out: Output,
    ) -> Self {
        PcMux {
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
            Some(mux_sel::pc::PC_PLUS4) => self.pc + Word::from(4u32),
            Some(mux_sel::pc::ALU_OUT) => self.alu_out,
            Some(mux_sel::pc::ALU_MOD2) => self.alu_out & Word::from(0xFFFFFFFEu32),
            _ => Word::unknown(),
        };

        send_word!(self, self.out, out);
    }
}
