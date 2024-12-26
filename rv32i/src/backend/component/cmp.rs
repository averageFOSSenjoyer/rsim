use crate::backend::util::byte::{ByteOrd, SignedOrd};
use crate::backend::util::event::ByteEvent;
use crate::backend::util::event::WordEvent;
use crate::backend::util::types::Word;
use crate::backend::util::types::*;
use crate::send_byte;
use crate::send_word;
use crossbeam_channel::Sender;
use rsim_core::component::Component;
use rsim_core::event::get_inner;
use rsim_core::sim_manager::SimManager;
use rsim_core::task::Task;
use rsim_core::types::ComponentId;
use rsim_core::types::EventId;
use rsim_core::types::Input;
use rsim_core::types::Output;
use rsim_core::{ack, enq, send};
use rsim_macro::ComponentAttribute;
use std::cmp::Ordering::Less;
use std::fmt::{Debug, Formatter};
use std::sync::Arc;

#[ComponentAttribute({
"port": {
    "input": [
        ["a", "Word"],
        ["b", "Word"],
        ["op", "Byte"]
    ],
    "output": [
        ["out_control", "Byte"],
        ["out_regfile_mux", "Word"]
    ]
}
})]
pub struct Cmp {}

impl Cmp {
    pub fn new(
        component_id: ComponentId,
        sim_manager: Arc<SimManager>,
        ack_sender: Sender<EventId>,
        a_receiver: Input,
        b_receiver: Input,
        op_receiver: Input,
        out_control: Output,
        out_regfile_mux: Output,
    ) -> Self {
        Cmp {
            component_id,
            sim_manager,
            ack_sender,
            a_receiver,
            a: Default::default(),
            a_old: Default::default(),
            b_receiver,
            b: Default::default(),
            b_old: Default::default(),
            op_receiver,
            op: Default::default(),
            op_old: Default::default(),
            out_control,
            out_regfile_mux,
        }
    }

    fn init_impl(&mut self) {}

    fn reset_impl(&mut self) {}

    fn poll_impl(&mut self) {}

    fn on_comb(&mut self) {
        if match self.op.into() {
            Some(funct3::branch::BEQ) => self.a == self.b,
            Some(funct3::branch::BNE) => self.a != self.b,
            Some(funct3::branch::BLT) => self.a.signed_cmp(self.b) == Less,
            Some(funct3::branch::BGE) => self.a.signed_cmp(self.b) != Less,
            Some(funct3::branch::BLTU) => self.a.byte_cmp(self.b) == Less,
            Some(funct3::branch::BGEU) => self.a.byte_cmp(self.b) != Less,
            _ => false,
        } {
            send_byte!(self, self.out_control, Byte::from(1u8));
            send_word!(self, self.out_regfile_mux, Word::from(1u32));
        } else {
            send_byte!(self, self.out_control, Byte::from(0u8));
            send_word!(self, self.out_regfile_mux, Word::from(0u32));
        };
    }
}

impl Debug for Cmp {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "CMP: {{a: {:?}, b: {:?}, op: {:?}}}",
            self.a, self.b, self.op
        )
    }
}

#[ComponentAttribute({
"port": {
    "input": [
        ["rs2", "Word"],
        ["i_imm", "Word"],
        ["sel", "Byte"]
    ],
    "output": [
        ["out", "Word"]
    ]
}
})]
pub struct CmpMux {}

impl CmpMux {
    pub fn new(
        component_id: ComponentId,
        sim_manager: Arc<SimManager>,
        ack_sender: Sender<EventId>,
        rs2_receiver: Input,
        i_imm_receiver: Input,
        sel_receiver: Input,
        out: Output,
    ) -> Self {
        CmpMux {
            component_id,
            sim_manager,
            ack_sender,
            rs2_receiver,
            rs2: Default::default(),
            rs2_old: Default::default(),
            i_imm_receiver,
            i_imm: Default::default(),
            i_imm_old: Default::default(),
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
            Some(mux_sel::cmp::RS2_OUT) => self.rs2,
            Some(mux_sel::cmp::I_IMM) => self.i_imm,
            _ => Word::unknown(),
        };

        send_word!(self, self.out, out);
    }
}
