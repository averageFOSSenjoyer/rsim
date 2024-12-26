use crate::byte::{ByteOrd, Bytes, Shra, SignedOrd};
use crate::control::ControlSignals::*;
use crate::control::States::Fetch2;
use crate::control::{ControlSignals, States};
use crate::ir::IR;
use crate::mem_ctl::MemCtl;
use crate::regfile::RegFile;
use crate::types::Word;
use crate::types::*;
use crate::util::sign_extend;
use std::cmp::Ordering::Less;
use std::collections::HashMap;
use std::process::exit;

#[derive(Debug)]
pub struct Core {
    pc: Word,
    mar: Word,
    mdr: Word,
    regfile_din: Word,
    alu_out: Word,
    cmp_out: Word,
    wmask: Byte,
    rmask: Byte,
    ir: IR,
    data_out: Word,
    control_signals: HashMap<ControlSignals, u8>,
    state: States,
    next_state: States,
    regfile: RegFile,
    mem_ctl: MemCtl,
}

impl Core {
    fn update_state(&mut self) {
        self.state = self.next_state;
    }

    fn load_pc(&mut self, sel: u8) {
        self.control_signals.insert(LoadPc, 1u8);
        self.control_signals.insert(PcMuxSel, sel);
    }

    fn load_regfile(&mut self, sel: u8) {
        self.control_signals.insert(LoadRegfile, 1u8);
        self.control_signals.insert(RegfileMuxSel, sel);
    }

    fn load_mar(&mut self, sel: u8) {
        self.control_signals.insert(LoadMar, 1u8);
        self.control_signals.insert(MarMuxSel, sel);
    }

    fn load_mdr(&mut self) {
        self.control_signals.insert(LoadMdr, 1u8);
        self.control_signals.insert(MemRead, 1u8);
        self.mem_ctl.req_access();
    }

    fn load_ir(&mut self) {
        self.control_signals.insert(LoadIr, 1u8);
    }

    fn load_dout(&mut self) {
        self.control_signals.insert(LoadDataOut, 1u8);
    }

    fn write_to_mem(&mut self, byte_enable: u8) {
        self.control_signals.insert(MemWrite, 1u8);
        self.control_signals.insert(MemByteEnable, byte_enable);
        self.mem_ctl.req_access();
    }

    fn set_alu(&mut self, sel1: u8, sel2: u8, alu_op: u8) {
        self.control_signals.insert(AluMux1Sel, sel1);
        self.control_signals.insert(AluMux2Sel, sel2);
        self.control_signals.insert(AluOp, alu_op);
    }

    fn set_cmp(&mut self, sel: u8, cmp_op: u8) {
        self.control_signals.insert(CmpMuxSel, sel);
        self.control_signals.insert(CmpOp, cmp_op);
    }

    fn set_default_control_signals(&mut self) {
        self.control_signals.insert(LoadMar, 0u8);
        self.control_signals.insert(LoadMdr, 0u8);
        self.control_signals.insert(LoadPc, 0u8);
        self.control_signals.insert(LoadIr, 0u8);
        self.control_signals.insert(LoadRegfile, 0u8);
        self.control_signals.insert(LoadDataOut, 0u8);
        self.control_signals.insert(PcMuxSel, 0u8);
        self.control_signals.insert(CmpOp, 0u8);
        self.control_signals.insert(AluMux1Sel, 0u8);
        self.control_signals.insert(AluMux2Sel, 0u8);
        self.control_signals.insert(RegfileMuxSel, 0u8);
        self.control_signals.insert(MarMuxSel, 0u8);
        self.control_signals.insert(CmpMuxSel, 0u8);
        self.control_signals.insert(AluOp, 0u8);
        self.control_signals.insert(MemRead, 0u8);
        self.control_signals.insert(MemWrite, 0u8);
        self.control_signals.insert(MemByteEnable, 0u8);
    }

    fn set_control_signals(&mut self) {
        self.set_default_control_signals();
        match self.state {
            States::Fetch1 => {
                self.load_mar(mux_sel::mar::PC_OUT);
            }
            States::Fetch2 => {
                self.load_mdr();
            }
            States::Fetch3 => {
                self.load_ir();
            }
            States::Decode => {}
            States::Imm => {
                if let (Some(funct3), Some(funct7)) = (
                    Into::<Option<u8>>::into(self.ir.funct3),
                    Into::<Option<u8>>::into(self.ir.funct7),
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
            States::Reg => {
                if let (Some(funct3), Some(funct7)) = (
                    Into::<Option<u8>>::into(self.ir.funct3),
                    Into::<Option<u8>>::into(self.ir.funct7),
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
            States::Lui => {
                self.load_regfile(mux_sel::regfile::U_IMM);
                self.load_pc(mux_sel::pc::PC_PLUS4);
            }
            States::Br => {
                self.set_alu(mux_sel::alu1::PC_OUT, mux_sel::alu2::B_IMM, alu_op::ADD);
                if let Some(funct3) = Into::<Option<u8>>::into(self.ir.funct3) {
                    self.set_cmp(mux_sel::cmp::RS2_OUT, funct3);
                }
                self.load_pc(if self.calc_cmp_out().is_something_nonzero() {
                    mux_sel::pc::ALU_OUT
                } else {
                    mux_sel::pc::PC_PLUS4
                });
            }
            States::Auipc => {
                self.load_regfile(mux_sel::regfile::ALU_OUT);
                self.set_alu(mux_sel::alu1::PC_OUT, mux_sel::alu2::U_IMM, alu_op::ADD);
                if let Some(funct3) = Into::<Option<u8>>::into(self.ir.funct3) {
                    self.set_cmp(mux_sel::cmp::RS2_OUT, funct3);
                }
                self.load_pc(mux_sel::pc::PC_PLUS4);
            }
            States::AddrCalc => {
                self.load_mar(mux_sel::mar::ALU_OUT);
                self.set_alu(
                    mux_sel::alu1::RS1_OUT,
                    if self.ir.opcode == Byte::from(opcode::LOAD) {
                        mux_sel::alu2::I_IMM
                    } else {
                        mux_sel::alu2::S_IMM
                    },
                    alu_op::ADD,
                );
                if self.ir.opcode == Byte::from(opcode::STORE) {
                    self.load_dout();
                }
            }
            States::Load1 => {
                self.load_mdr();
            }
            States::Load2 => {
                if let Some(funct3) = Into::<Option<u8>>::into(self.ir.funct3) {
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
            States::Store1 => {
                if let Some(wmask) = Into::<Option<u8>>::into(self.wmask) {
                    self.write_to_mem(wmask);
                }
            }
            States::Store2 => {
                self.load_pc(mux_sel::pc::PC_PLUS4);
            }
            States::Jal => {
                self.load_pc(mux_sel::pc::ALU_OUT);
                self.set_alu(mux_sel::alu1::PC_OUT, mux_sel::alu2::J_IMM, alu_op::ADD);
                self.load_regfile(mux_sel::regfile::PC_PLUS4);
            }
            States::Jalr => {
                self.load_pc(mux_sel::pc::ALU_MOD2);
                self.set_alu(mux_sel::alu1::RS1_OUT, mux_sel::alu2::I_IMM, alu_op::ADD);
                self.load_regfile(mux_sel::regfile::PC_PLUS4);
            }
        }
    }

    fn calc_alu_out(&self) -> Word {
        let a = match self.control_signals[&AluMux1Sel] {
            mux_sel::alu1::RS1_OUT => self.regfile.read(self.ir.rs1),
            mux_sel::alu1::PC_OUT => self.pc,
            _ => Word::unknown(),
        };

        let b = match self.control_signals[&AluMux2Sel] {
            mux_sel::alu2::I_IMM => self.ir.i_imm,
            mux_sel::alu2::U_IMM => self.ir.u_imm,
            mux_sel::alu2::B_IMM => self.ir.b_imm,
            mux_sel::alu2::S_IMM => self.ir.s_imm,
            mux_sel::alu2::J_IMM => self.ir.j_imm,
            mux_sel::alu2::RS2_OUT => self.regfile.read(self.ir.rs2),
            _ => Word::unknown(),
        };

        match self.control_signals[&AluOp] {
            alu_op::ADD => a + b,
            alu_op::SLL => a << (b & Word::from(0x1Fu32)),
            alu_op::SRA => a.shra(b & Word::from(0x1Fu32)),
            alu_op::SUB => a - b,
            alu_op::XOR => a ^ b,
            alu_op::SRL => a >> (b & Word::from(0x1Fu32)),
            alu_op::OR => a | b,
            alu_op::AND => a & b,
            _ => Word::unknown(),
        }
    }

    fn calc_cmp_out(&self) -> Word {
        let a = self.regfile.read(self.ir.rs1);
        let b = match self.control_signals[&CmpMuxSel] {
            mux_sel::cmp::RS2_OUT => self.regfile.read(self.ir.rs2),
            mux_sel::cmp::I_IMM => self.ir.i_imm,
            _ => Word::unknown(),
        };

        if match self.control_signals[&CmpOp] {
            funct3::branch::BEQ => a == b,
            funct3::branch::BNE => a != b,
            funct3::branch::BLT => a.signed_cmp(b) == Less,
            funct3::branch::BGE => a.signed_cmp(b) != Less,
            funct3::branch::BLTU => a.byte_cmp(b) == Less,
            funct3::branch::BGEU => a.byte_cmp(b) != Less,
            _ => false,
        } {
            Word::from(1u32)
        } else {
            Word::from(0u32)
        }
    }

    fn calc_wmask(&self) -> Byte {
        if let Some(funct3) = Into::<Option<u8>>::into(self.ir.funct3) {
            match funct3 {
                funct3::store::SW => Byte::from(0x0Fu8),
                funct3::store::SH => Byte::from(0x03u8) << (self.mar & Byte::from(0x3u8)),
                funct3::store::SB => Byte::from(0x01u8) << (self.mar & Byte::from(0x3u8)),
                _ => Byte::unknown(),
            }
        } else {
            Byte::unknown()
        }
    }

    fn calc_rmask(&self) -> Byte {
        if self.state == Fetch2 {
            Byte::from(0x0Fu8)
        } else if let Some(funct3) = Into::<Option<u8>>::into(self.ir.funct3) {
            match funct3 {
                funct3::load::LW => Byte::from(0x0Fu8),
                funct3::load::LH | funct3::load::LHU => {
                    Byte::from(0x03u8) << (self.mar & Byte::from(0x3u8))
                }
                funct3::load::LB | funct3::load::LBU => {
                    Byte::from(0x01u8) << (self.mar & Byte::from(0x3u8))
                }
                _ => Byte::unknown(),
            }
        } else {
            Byte::unknown()
        }
    }

    fn update_mar(&mut self) {
        let mar_mux_sel = self.control_signals[&MarMuxSel];
        self.mar = if mar_mux_sel == mux_sel::mar::PC_OUT {
            self.pc
        } else if mar_mux_sel == mux_sel::mar::ALU_OUT {
            self.alu_out
        } else {
            Word::unknown()
        }
    }

    fn update_mdr(&mut self) {
        self.mdr = if self.mem_ctl.is_resp() {
            self.mem_ctl
                .read(&(self.mar & Word::from(0xFFFFFFFCu32)), self.rmask)
        } else {
            Word::unknown()
        }
    }

    fn update_pc(&mut self) {
        self.pc = match self.control_signals[&PcMuxSel] {
            mux_sel::pc::PC_PLUS4 => self.pc + Word::from(4u32),
            mux_sel::pc::ALU_OUT => self.alu_out,
            mux_sel::pc::ALU_MOD2 => self.alu_out & Word::from(0xFFFFFFFEu32),
            _ => self.pc,
        };
    }

    fn update_ir(&mut self) {
        self.ir.write(self.mdr);
    }

    fn calc_regfile_din(&self) -> Word {
        let mdr_idx = (Into::<Option<u32>>::into(self.mar).unwrap_or(0) & 0x3u32) as usize;
        match self.control_signals[&RegfileMuxSel] {
            mux_sel::regfile::ALU_OUT => self.alu_out,
            mux_sel::regfile::BR_EN => self.cmp_out,
            mux_sel::regfile::U_IMM => self.ir.u_imm,
            mux_sel::regfile::LW => self.mdr,
            mux_sel::regfile::PC_PLUS4 => self.pc + Word::from(4u32),
            mux_sel::regfile::LB => {
                let val = self.mdr[mdr_idx]
                    .map(|byte| Word::from(byte as u32))
                    .unwrap_or(Word::unknown());
                if !val.has_unknown() {
                    sign_extend(Into::<Option<u32>>::into(val).unwrap(), 7)
                } else {
                    Word::unknown()
                }
            }
            mux_sel::regfile::LBU => self.mdr[mdr_idx]
                .map(|byte| Word::from(byte as u32))
                .unwrap_or(Word::unknown()),
            mux_sel::regfile::LH => {
                let val = self.mdr[mdr_idx]
                    .map(|lsb| {
                        self.mdr[mdr_idx + 1]
                            .map(|msb| Word::from((((msb as u16) << 8) | lsb as u16) as u32))
                            .unwrap_or(Word::unknown())
                    })
                    .unwrap_or(Word::unknown());
                if !val.has_unknown() {
                    sign_extend(Into::<Option<u32>>::into(val).unwrap(), 15)
                } else {
                    Word::unknown()
                }
            }
            mux_sel::regfile::LHU => self.mdr[mdr_idx]
                .map(|lsb| {
                    self.mdr[mdr_idx + 1]
                        .map(|msb| Word::from((((msb as u16) << 8) | lsb as u16) as u32))
                        .unwrap_or(Word::unknown())
                })
                .unwrap_or(Word::unknown()),
            _ => Word::unknown(),
        }
    }

    fn update_regfile(&mut self) {
        self.regfile.write(self.ir.rd, self.regfile_din);
    }

    fn update_dataout(&mut self) {
        if let Some(mar) = Into::<Option<u32>>::into(self.mar) {
            self.data_out = self.regfile.read(self.ir.rs2) << Word::from(8 * (mar & 0x3));
        }
    }

    fn update_comb_signal(&mut self) {
        self.mem_ctl.tick();
        self.alu_out = self.calc_alu_out();
        self.cmp_out = self.calc_cmp_out();
        self.regfile_din = self.calc_regfile_din();
        self.wmask = self.calc_wmask();
        self.rmask = self.calc_rmask();
    }

    fn update_datapath(&mut self) {
        if self.control_signals[&LoadMar] != 0 {
            self.update_mar()
        }

        if self.control_signals[&LoadMdr] != 0 {
            self.update_mdr()
        }

        if self.control_signals[&LoadPc] != 0 {
            self.update_pc()
        }

        if self.control_signals[&LoadIr] != 0 {
            self.update_ir()
        }

        if self.control_signals[&LoadRegfile] != 0 {
            self.update_regfile()
        }

        if self.control_signals[&LoadDataOut] != 0 {
            self.update_dataout()
        }

        if self.mem_ctl.is_resp() && self.control_signals[&MemWrite] != 0 {
            self.mem_ctl.write(
                &(self.mar & Word::from(0xFFFFFFFCu32)),
                self.data_out,
                self.wmask,
            )
        }
    }

    fn set_next_state(&mut self) {
        self.next_state = self.state.clone();

        self.next_state = match self.state {
            States::Fetch1 => States::Fetch2,
            States::Fetch2 => {
                if self.mem_ctl.is_resp() {
                    States::Fetch3
                } else {
                    States::Fetch2
                }
            }
            States::Fetch3 => States::Decode,
            States::Decode => match Into::<Option<u8>>::into(self.ir.opcode) {
                Some(opcode::LUI) => States::Lui,
                Some(opcode::AUIPC) => States::Auipc,
                Some(opcode::JAL) => States::Jal,
                Some(opcode::JALR) => States::Jalr,
                Some(opcode::BR) => States::Br,
                Some(opcode::LOAD) | Some(opcode::STORE) => States::AddrCalc,
                Some(opcode::IMM) => States::Imm,
                Some(opcode::REG) => States::Reg,
                _ => States::Fetch1,
            },
            States::AddrCalc => {
                if self.ir.opcode == Byte::from(opcode::LOAD) {
                    States::Load1
                } else {
                    States::Store1
                }
            }
            States::Load1 => {
                if self.mem_ctl.is_resp() {
                    States::Load2
                } else {
                    States::Load1
                }
            }
            States::Store1 => {
                if self.mem_ctl.is_resp() {
                    States::Store2
                } else {
                    States::Store1
                }
            }
            _ => States::Fetch1,
        }
    }

    fn write_spike_log(&self) {
        if self.state == States::Fetch1
            || self.state == States::Fetch2
            || self.state == States::Fetch3
            || self.state == States::Decode
            || self.state == States::Store2
            || self.state == States::Load1
            || self.state == States::AddrCalc
        {
            return;
        }

        // if Into::<Option<u32>>::into(self.pc).unwrap() == 0x40000118u32 {
        //     println!("{:?}", self);
        //     println!("{:?}", self.calc_alu_out());
        //     exit(0);
        // }

        let mut line = String::new();

        line.push_str(&format!("core   0: 3 0x{} (0x{})", self.pc, self.ir.data));

        if self.control_signals[&LoadRegfile] != 0 && self.ir.rd.is_something_nonzero() {
            let raw_rd: u8 = Into::<Option<u8>>::into(self.ir.rd).unwrap();
            if raw_rd < 10 {
                line.push_str(&format!(" x{}  ", raw_rd))
            } else {
                line.push_str(&format!(" x{} ", raw_rd))
            }
            line.push_str(&format!("0x{}", self.regfile_din));
        }

        if self.state == States::Load2 && self.rmask.is_something_nonzero() {
            let rmask = Into::<Option<u8>>::into(self.rmask).unwrap();
            let mut byte_shift = 0;
            for i in 0..4u8 {
                if (rmask >> i) & 0x1 == 0x1 {
                    byte_shift = i;
                    break;
                }
            }
            line.push_str(&format!(
                " mem 0x{}",
                self.mar & Word::from(0xFFFFFFFCu32) + Byte::from(byte_shift)
            ));
        }

        if self.control_signals[&MemWrite] != 0 && self.wmask.is_something_nonzero() {
            let wmask = Into::<Option<u8>>::into(self.wmask).unwrap();
            let mut byte_shift = 0;
            for i in 0..4u8 {
                if (wmask >> i) & 0x1 == 0x1 {
                    byte_shift = i;
                    break;
                }
            }
            let mut byte_count = 0;
            for i in 0..4u8 {
                if (wmask >> i) & 0x1 == 0x1 {
                    byte_count += 1;
                }
            }

            line.push_str(&format!(
                " mem 0x{}",
                self.mar & Word::from(0xFFFFFFFCu32) + Byte::from(byte_shift)
            ));
            if let Some(data_out) = Into::<Option<u32>>::into(self.data_out) {
                let shifted_data = data_out >> (8 * byte_shift);
                let data_out_str = match byte_count {
                    1 => {
                        format!("{}", Byte::from(shifted_data as u8))
                    }
                    2 => {
                        format!("{}", Bytes::<2>::from(shifted_data as u16))
                    }
                    4 => {
                        format!("{}", Word::from(shifted_data))
                    }
                    _ => "".to_string(),
                };
                line.push_str(&format!(" 0x{}", data_out_str));
            }
        }

        line.push_str("\n");
        print!("{}", line);
        if self.regfile_din.has_unknown() {
            exit(0);
        }
    }

    pub fn next_cycle(&mut self) {
        self.update_state();
        self.set_control_signals();
        self.update_comb_signal();
        self.write_spike_log();
        self.update_datapath();
        self.set_next_state();
    }

    pub fn load_bin(&mut self, data: &Vec<u8>, addr: Word) {
        self.mem_ctl.load_bin(data, addr)
    }

    pub fn should_halt(&self) -> bool {
        (self.state == States::Imm && self.ir.data == Word::from(0xF0002013u32))
            || (self.state == States::Br && self.ir.data == Word::from(0x00000063u32))
    }
}

impl Default for Core {
    fn default() -> Self {
        Self {
            pc: Word::from(0x40000000u32),
            mar: Word::unknown(),
            mdr: Word::unknown(),
            regfile_din: Default::default(),
            alu_out: Default::default(),
            cmp_out: Default::default(),
            wmask: Default::default(),
            rmask: Default::default(),
            ir: IR::default(),
            data_out: Word::unknown(),
            control_signals: Default::default(),
            state: States::Fetch1,
            next_state: States::Fetch1,
            regfile: RegFile::default(),
            mem_ctl: MemCtl::default(),
        }
    }
}
