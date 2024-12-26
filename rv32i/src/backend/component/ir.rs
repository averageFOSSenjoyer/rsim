use crate::backend::util::event::{ByteEvent, WordEvent};
use crate::backend::util::helper::sign_extend;
use crate::backend::util::types::Byte;
use crate::backend::util::types::Word;
use crate::send_byte;
use crate::send_word;
use crossbeam_channel::{unbounded, Sender};
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
        ["funct3", "Byte"],
        ["funct7", "Byte"],
        ["opcode", "Byte"],
        ["i_imm_alu_mux2", "Word"],
        ["i_imm_cmp_mux", "Word"],
        ["s_imm", "Word"],
        ["b_imm", "Word"],
        ["u_imm_alu_mux2", "Word"],
        ["u_imm_regfile_mux", "Word"],
        ["j_imm", "Word"],
        ["rs1", "Byte"],
        ["rs2", "Byte"],
        ["rd", "Byte"]
    ],
    "clock": true
}
})]
pub struct IR {
    pub data_inner: Word,
}

impl IR {
    pub fn new(
        component_id: ComponentId,
        sim_manager: Arc<SimManager>,
        ack_sender: Sender<EventId>,
        load_receiver: Input,
        data_receiver: Input,
        funct3: Output,
        funct7: Output,
        opcode: Output,
        i_imm_alu_mux2: Output,
        i_imm_cmp_mux: Output,
        s_imm: Output,
        b_imm: Output,
        u_imm_alu_mux2: Output,
        u_imm_regfile_mux: Output,
        j_imm: Output,
        rs1: Output,
        rs2: Output,
        rd: Output,
    ) -> Self {
        let clock_channel = unbounded();

        IR {
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
            funct3,
            funct7,
            opcode,
            i_imm_alu_mux2,
            i_imm_cmp_mux,
            s_imm,
            b_imm,
            u_imm_alu_mux2,
            u_imm_regfile_mux,
            j_imm,
            rs1,
            rs2,
            rd,
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

    pub fn get_rd_idx(&self) -> Byte {
        if let Some(inst) = Into::<Option<u32>>::into(self.data_inner) {
            Byte::from(((inst >> 7) & 0x1F) as u8)
        } else {
            Byte::unknown()
        }
    }

    fn on_comb(&mut self) {
        if let Some(inst) = Into::<Option<u32>>::into(self.data_inner) {
            send_byte!(self, self.funct3, Byte::from(((inst >> 12) & 0b111) as u8));
            send_byte!(
                self,
                self.funct7,
                Byte::from(((inst >> 25) & 0b1111111) as u8)
            );
            send_byte!(self, self.opcode, Byte::from((inst & 0b1111111) as u8));
            send_word!(self, self.i_imm_alu_mux2, sign_extend(inst >> 20, 11));
            send_word!(self, self.i_imm_cmp_mux, sign_extend(inst >> 20, 11));
            send_word!(
                self,
                self.s_imm,
                sign_extend(
                    (((inst >> 25) & 0b1111111) << 5) | ((inst >> 7) & 0b11111),
                    11,
                )
            );
            send_word!(
                self,
                self.b_imm,
                sign_extend(
                    (((inst >> 31) & 0b1) << 12)
                        | (((inst >> 7) & 0b1) << 11)
                        | (((inst >> 25) & 0b111111) << 5)
                        | (((inst >> 8) & 0b1111) << 1),
                    12,
                )
            );
            send_word!(
                self,
                self.u_imm_alu_mux2,
                Word::from(((inst >> 12) & 0xFFFFF) << 12)
            );
            send_word!(
                self,
                self.u_imm_regfile_mux,
                Word::from(((inst >> 12) & 0xFFFFF) << 12)
            );
            send_word!(
                self,
                self.j_imm,
                sign_extend(
                    (((inst >> 31) & 0b1) << 20)
                        | (((inst >> 12) & 0xFF) << 12)
                        | (((inst >> 20) & 0b1) << 11)
                        | (((inst >> 21) & 0x3FF) << 1),
                    20,
                )
            );
            send_byte!(self, self.rs1, Byte::from(((inst >> 15) & 0x1F) as u8));
            send_byte!(self, self.rs2, Byte::from(((inst >> 20) & 0x1F) as u8));
            send_byte!(self, self.rd, Byte::from(((inst >> 7) & 0x1F) as u8));
        }
    }

    pub fn can_end(&self) -> bool {
        self.data_inner == Word::from(0xF0002013u32) || self.data_inner == Word::from(0x00000063u32)
    }
}

impl Debug for IR {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "IR: {{{:?}}}", self.data_inner.clone())
    }
}
