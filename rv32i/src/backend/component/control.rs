use crate::backend::util::event::ByteEvent;
use crate::backend::util::types::States::*;
use crate::backend::util::types::*;
use crate::send_byte;
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
        ["funct3", "Byte"],
        ["funct7", "Byte"],
        ["cmp_out", "Byte"],
        ["opcode", "Byte"],
        ["mar", "Word"],
        ["mem_resp", "Byte"]
    ],
    "output": [
        ["load_mar", "Byte"],
        ["load_mdr", "Byte"],
        ["load_pc", "Byte"],
        ["load_ir", "Byte"],
        ["load_regfile", "Byte"],
        ["load_dataout", "Byte"],
        ["alu_op", "Byte"],
        ["cmp_op", "Byte"],
        ["pc_mux_sel", "Byte"],
        ["alu_mux1_sel", "Byte"],
        ["alu_mux2_sel", "Byte"],
        ["regfile_mux_sel", "Byte"],
        ["mar_mux_sel", "Byte"],
        ["cmp_mux_sel", "Byte"],
        ["mem_read", "Byte"],
        ["mem_write", "Byte"],
        ["mem_wmask", "Byte"],
        ["mem_rmask", "Byte"]
    ],
    "clock": true
}
})]
pub struct Control {
    pub state: States,
    pub next_state: States,
}

impl Control {
    pub fn new(
        component_id: ComponentId,
        sim_manager: Arc<SimManager>,
        ack_sender: Sender<EventId>,
        funct3_receiver: Input,
        funct7_receiver: Input,
        cmp_out_receiver: Input,
        opcode_receiver: Input,
        mar_receiver: Input,
        mem_resp_receiver: Input,
        load_mar: Output,
        load_mdr: Output,
        load_pc: Output,
        load_ir: Output,
        load_regfile: Output,
        load_dataout: Output,
        alu_op: Output,
        cmp_op: Output,
        pc_mux_sel: Output,
        alu_mux1_sel: Output,
        alu_mux2_sel: Output,
        regfile_mux_sel: Output,
        mar_mux_sel: Output,
        cmp_mux_sel: Output,
        mem_read: Output,
        mem_write: Output,
        mem_wmask: Output,
        mem_rmask: Output,
    ) -> Self {
        let clock_channel = unbounded();

        Control {
            state: Fetch1,
            next_state: Fetch1,
            component_id,
            sim_manager,
            ack_sender,
            clock_sender: clock_channel.0,
            clock_receiver: clock_channel.1,
            funct3_receiver,
            funct3: Default::default(),
            funct3_old: Default::default(),
            funct7_receiver,
            funct7: Default::default(),
            funct7_old: Default::default(),
            cmp_out_receiver,
            cmp_out: Default::default(),
            cmp_out_old: Default::default(),
            opcode_receiver,
            opcode: Default::default(),
            opcode_old: Default::default(),
            mar_receiver,
            mar: Default::default(),
            mar_old: Default::default(),
            mem_resp_receiver,
            mem_resp: Default::default(),
            mem_resp_old: Default::default(),
            load_mar,
            load_mdr,
            load_pc,
            load_ir,
            load_regfile,
            load_dataout,
            alu_op,
            cmp_op,
            pc_mux_sel,
            alu_mux1_sel,
            alu_mux2_sel,
            regfile_mux_sel,
            mar_mux_sel,
            cmp_mux_sel,
            mem_read,
            mem_write,
            mem_wmask,
            mem_rmask,
        }
    }

    fn init_impl(&mut self) {}

    fn reset_impl(&mut self) {}

    fn poll_impl(&mut self) {}

    fn on_clock(&mut self) {
        self.state = self.next_state;
    }

    pub fn get_rmask(&self) -> Byte {
        if self.state == Fetch2 {
            Byte::from(0x0Fu8)
        } else {
            match self.funct3.into() {
                Some(funct3::load::LW) => Byte::from(0x0Fu8),
                Some(funct3::load::LH) | Some(funct3::load::LHU) => {
                    Byte::from(0x03u8) << (self.mar & Byte::from(0x3u8))
                }
                Some(funct3::load::LB) | Some(funct3::load::LBU) => {
                    Byte::from(0x01u8) << (self.mar & Byte::from(0x3u8))
                }
                _ => Byte::unknown(),
            }
        }
    }

    pub fn get_wmask(&self) -> Byte {
        match self.funct3.into() {
            Some(funct3::store::SW) => Byte::from(0x0Fu8),
            Some(funct3::store::SH) => Byte::from(0x03u8) << (self.mar & Byte::from(0x3u8)),
            Some(funct3::store::SB) => Byte::from(0x01u8) << (self.mar & Byte::from(0x3u8)),
            _ => Byte::unknown(),
        }
    }

    fn set_default_control_signals(&mut self) {
        send_byte!(self, self.load_mar, Byte::from(0u8));
        send_byte!(self, self.load_mdr, Byte::from(0u8));
        send_byte!(self, self.load_pc, Byte::from(0u8));
        send_byte!(self, self.load_ir, Byte::from(0u8));
        send_byte!(self, self.load_regfile, Byte::from(0u8));
        send_byte!(self, self.load_dataout, Byte::from(0u8));
        // send_byte!(self, self.pc_mux_sel, Byte::from(0u8));
        // send_byte!(self, self.cmp_op, Byte::from(0u8));
        // send_byte!(self, self.alu_mux1_sel, Byte::from(0u8));
        // send_byte!(self, self.alu_mux2_sel, Byte::from(0u8));
        // send_byte!(self, self.regfile_mux_sel, Byte::from(0u8));
        // send_byte!(self, self.mar_mux_sel, Byte::from(0u8));
        // send_byte!(self, self.cmp_mux_sel, Byte::from(0u8));
        // send_byte!(self, self.alu_op, Byte::from(0u8));
        send_byte!(self, self.mem_read, Byte::from(0u8));
        send_byte!(self, self.mem_write, Byte::from(0u8));
        send_byte!(self, self.mem_wmask, self.get_wmask());
        send_byte!(self, self.mem_rmask, self.get_rmask());
    }

    fn load_pc(&mut self, sel: u8) {
        send_byte!(self, self.load_pc, Byte::from(1u8));
        send_byte!(self, self.pc_mux_sel, Byte::from(sel));
    }

    fn load_regfile(&mut self, sel: u8) {
        send_byte!(self, self.load_regfile, Byte::from(1u8));
        send_byte!(self, self.regfile_mux_sel, Byte::from(sel));
    }

    fn load_mar(&mut self, sel: u8) {
        send_byte!(self, self.load_mar, Byte::from(1u8));
        send_byte!(self, self.mar_mux_sel, Byte::from(sel));
    }

    fn load_ir(&mut self) {
        send_byte!(self, self.load_ir, Byte::from(1u8));
    }

    fn load_dout(&mut self) {
        send_byte!(self, self.load_dataout, Byte::from(1u8));
    }

    fn set_alu(&mut self, sel1: u8, sel2: u8, alu_op: u8) {
        send_byte!(self, self.alu_mux1_sel, Byte::from(sel1));
        send_byte!(self, self.alu_mux2_sel, Byte::from(sel2));
        send_byte!(self, self.alu_op, Byte::from(alu_op));
    }

    fn set_cmp(&mut self, sel: u8, cmp_op: u8) {
        send_byte!(self, self.cmp_mux_sel, Byte::from(sel));
        send_byte!(self, self.cmp_op, Byte::from(cmp_op));
    }

    fn read_from_mem(&mut self) {
        send_byte!(self, self.load_mdr, Byte::from(1u8));
        send_byte!(self, self.mem_read, Byte::from(1u8));
    }

    fn write_to_mem(&mut self) {
        send_byte!(self, self.mem_write, Byte::from(1u8));
    }

    fn set_control_signal(&mut self) {
        match self.state {
            Fetch1 => {
                self.load_mar(mux_sel::mar::PC_OUT);
            }
            Fetch2 => {
                self.read_from_mem();
            }
            Fetch3 => {
                self.load_ir();
            }
            Decode => {}
            Imm => {
                if let (Some(funct3), Some(funct7)) = (
                    Into::<Option<u8>>::into(self.funct3),
                    Into::<Option<u8>>::into(self.funct7),
                ) {
                    match funct3 {
                        funct3::arith::SLT => {
                            self.load_regfile(mux_sel::regfile::BR_EN);
                            self.set_cmp(mux_sel::cmp::I_IMM, funct3::branch::BLT);
                        }
                        funct3::arith::SLTU => {
                            self.load_regfile(mux_sel::regfile::BR_EN);
                            self.set_cmp(mux_sel::cmp::I_IMM, funct3::branch::BLTU);
                        }
                        funct3::arith::SR => {
                            self.load_regfile(mux_sel::regfile::ALU_OUT);
                            self.set_alu(
                                mux_sel::alu1::RS1_OUT,
                                mux_sel::alu2::I_IMM,
                                if (funct7 >> 5) & 0x1 == 0x1 {
                                    alu_op::SRA
                                } else {
                                    alu_op::SRL
                                },
                            );
                        }
                        _ => {
                            self.load_regfile(mux_sel::regfile::ALU_OUT);
                            self.set_alu(mux_sel::alu1::RS1_OUT, mux_sel::alu2::I_IMM, funct3);
                        }
                    }
                }
                self.load_pc(mux_sel::pc::PC_PLUS4);
            }
            Reg => {
                if let (Some(funct3), Some(funct7)) = (
                    Into::<Option<u8>>::into(self.funct3),
                    Into::<Option<u8>>::into(self.funct7),
                ) {
                    match funct3 {
                        funct3::arith::ADD => {
                            self.load_regfile(mux_sel::regfile::ALU_OUT);
                            self.set_alu(
                                mux_sel::alu1::RS1_OUT,
                                mux_sel::alu2::RS2_OUT,
                                if (funct7 >> 5) & 0x1 == 0x1 {
                                    alu_op::SUB
                                } else {
                                    alu_op::ADD
                                },
                            );
                        }
                        funct3::arith::SR => {
                            self.load_regfile(mux_sel::regfile::ALU_OUT);
                            self.set_alu(
                                mux_sel::alu1::RS1_OUT,
                                mux_sel::alu2::RS2_OUT,
                                if (funct7 >> 5) & 0x1 == 0x1 {
                                    alu_op::SRA
                                } else {
                                    alu_op::SRL
                                },
                            );
                        }
                        funct3::arith::SLT => {
                            self.load_regfile(mux_sel::regfile::BR_EN);
                            self.set_cmp(mux_sel::cmp::RS2_OUT, funct3::branch::BLT);
                        }
                        funct3::arith::SLTU => {
                            self.load_regfile(mux_sel::regfile::BR_EN);
                            self.set_cmp(mux_sel::cmp::RS2_OUT, funct3::branch::BLTU);
                        }
                        _ => {
                            self.load_regfile(mux_sel::regfile::ALU_OUT);
                            self.set_alu(mux_sel::alu1::RS1_OUT, mux_sel::alu2::RS2_OUT, funct3);
                        }
                    }
                }
                self.load_pc(mux_sel::pc::PC_PLUS4);
            }
            Lui => {
                self.load_regfile(mux_sel::regfile::U_IMM);
                self.load_pc(mux_sel::pc::PC_PLUS4);
            }
            Br => {
                self.set_alu(mux_sel::alu1::PC_OUT, mux_sel::alu2::B_IMM, alu_op::ADD);
                if let Some(funct3) = Into::<Option<u8>>::into(self.funct3) {
                    self.set_cmp(mux_sel::cmp::RS2_OUT, funct3);
                }
                self.load_pc(if self.cmp_out.is_something_nonzero() {
                    mux_sel::pc::ALU_OUT
                } else {
                    mux_sel::pc::PC_PLUS4
                });
            }
            Auipc => {
                self.load_regfile(mux_sel::regfile::ALU_OUT);
                self.set_alu(mux_sel::alu1::PC_OUT, mux_sel::alu2::U_IMM, alu_op::ADD);
                if let Some(funct3) = Into::<Option<u8>>::into(self.funct3) {
                    self.set_cmp(mux_sel::cmp::RS2_OUT, funct3);
                }
                self.load_pc(mux_sel::pc::PC_PLUS4);
            }
            AddrCalc => {
                self.load_mar(mux_sel::mar::ALU_OUT);
                self.set_alu(
                    mux_sel::alu1::RS1_OUT,
                    if self.opcode == Byte::from(opcode::LOAD) {
                        mux_sel::alu2::I_IMM
                    } else {
                        mux_sel::alu2::S_IMM
                    },
                    alu_op::ADD,
                );
                if self.opcode == Byte::from(opcode::STORE) {
                    self.load_dout();
                }
            }
            Load1 => {
                self.read_from_mem();
            }
            Load2 => {
                if let Some(funct3) = Into::<Option<u8>>::into(self.funct3) {
                    match funct3 {
                        funct3::load::LB => {
                            self.load_regfile(mux_sel::regfile::LB);
                        }
                        funct3::load::LH => {
                            self.load_regfile(mux_sel::regfile::LH);
                        }
                        funct3::load::LW => {
                            self.load_regfile(mux_sel::regfile::LW);
                        }
                        funct3::load::LBU => {
                            self.load_regfile(mux_sel::regfile::LBU);
                        }
                        funct3::load::LHU => {
                            self.load_regfile(mux_sel::regfile::LHU);
                        }
                        _ => {}
                    }
                }
                self.load_pc(mux_sel::pc::PC_PLUS4);
            }
            Store1 => {
                self.write_to_mem();
            }
            Store2 => {
                self.load_pc(mux_sel::pc::PC_PLUS4);
            }
            Jal => {
                self.load_pc(mux_sel::pc::ALU_OUT);
                self.set_alu(mux_sel::alu1::PC_OUT, mux_sel::alu2::J_IMM, alu_op::ADD);
                self.load_regfile(mux_sel::regfile::PC_PLUS4);
            }
            Jalr => {
                self.load_pc(mux_sel::pc::ALU_MOD2);
                self.set_alu(mux_sel::alu1::RS1_OUT, mux_sel::alu2::I_IMM, alu_op::ADD);
                self.load_regfile(mux_sel::regfile::PC_PLUS4);
            }
        }
    }

    fn set_next_state(&mut self) {
        self.next_state = self.state;

        self.next_state = match self.state {
            Fetch1 => Fetch2,
            Fetch2 => {
                if self.mem_resp.is_something_nonzero() {
                    Fetch3
                } else {
                    Fetch2
                }
            }
            Fetch3 => Decode,
            Decode => match Into::<Option<u8>>::into(self.opcode) {
                Some(opcode::LUI) => Lui,
                Some(opcode::AUIPC) => Auipc,
                Some(opcode::JAL) => Jal,
                Some(opcode::JALR) => Jalr,
                Some(opcode::BR) => Br,
                Some(opcode::LOAD) | Some(opcode::STORE) => AddrCalc,
                Some(opcode::IMM) => Imm,
                Some(opcode::REG) => Reg,
                _ => Fetch1,
            },
            AddrCalc => {
                if self.opcode == Byte::from(opcode::LOAD) {
                    Load1
                } else {
                    Store1
                }
            }
            Load1 => {
                if self.mem_resp.is_something_nonzero() {
                    Load2
                } else {
                    Load1
                }
            }
            Store1 => {
                if self.mem_resp.is_something_nonzero() {
                    Store2
                } else {
                    Store1
                }
            }
            _ => Fetch1,
        }
    }

    fn on_comb(&mut self) {
        self.set_default_control_signals();
        self.set_control_signal();
        self.set_next_state();
    }
}

impl Debug for Control {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Control {{State: {:?}, funct3: {:?}}}",
            self.state, self.funct3
        )
    }
}
