use crate::backend::util::byte::Bytes;
use crate::backend::util::event::WordEvent;
use crate::backend::util::helper::sign_extend;
use crate::backend::util::types::*;
use crate::send_word;
use crossbeam_channel::{unbounded, Sender};
use rsim_core::component::Component;
use rsim_core::event::get_inner;
use rsim_core::send;
use rsim_core::sim_manager::SimManager;
use rsim_core::task::Task;
use rsim_core::types::ComponentId;
use rsim_core::types::EventId;
use rsim_core::types::Input;
use rsim_core::types::Output;
use rsim_core::{ack, enq};
use rsim_macro::ComponentAttribute;
use std::fmt::{Debug, Formatter};
use std::sync::Arc;

#[derive(Debug, Clone, Copy)]
pub struct Registers {
    data: [Word; 32],
}

impl Registers {
    fn read(&self, index: Bytes<1>) -> Word {
        Into::<Option<u8>>::into(index)
            .map(|idx| {
                if idx != 0 {
                    self.data[idx as usize]
                } else {
                    Word::zeros()
                }
            })
            .unwrap_or(Word::unknown())
    }

    fn write(&mut self, index: Bytes<1>, value: Word) {
        if let Some(idx) = Into::<Option<u8>>::into(index) {
            if idx != 0 {
                self.data[idx as usize] = value
            }
        }
    }

    fn reset(&mut self) {
        self.data = [Word::unknown(); 32];
        self.data[0] = Word::zeros();
    }
}

impl Default for Registers {
    fn default() -> Self {
        let mut data = [Word::zeros(); 32];
        data[0] = Word::zeros();
        Self { data }
    }
}

/// A registerfile, consists of 32 4-byte registers
#[ComponentAttribute({
"port": {
    "input": [
        ["rs1_idx", "Byte"],
        ["rs2_idx", "Byte"],
        ["rd_wr", "Byte"],
        ["rd_idx", "Byte"],
        ["rd_data", "Word"]
    ],
    "output": [
        ["rs1_data_alu_mux1", "Word"],
        ["rs1_data_cmp", "Word"],
        ["rs2_data_alu_mux2", "Word"],
        ["rs2_data_cmp_mux", "Word"],
        ["rs2_data_data_out", "Word"]
    ],
    "clock": true
}
})]
pub struct RegFile {
    pub registers: Registers,
}

impl RegFile {
    pub fn new(
        component_id: ComponentId,
        sim_manager: Arc<SimManager>,
        ack_sender: Sender<EventId>,
        rs1_idx_receiver: Input,
        rs2_idx_receiver: Input,
        rd_wr_receiver: Input,
        rd_idx_receiver: Input,
        rd_data_receiver: Input,
        rs1_data_alu_mux1: Output,
        rs1_data_cmp: Output,
        rs2_data_alu_mux2: Output,
        rs2_data_cmp_mux: Output,
        rs2_data_data_out: Output,
    ) -> Self {
        let clock_channel = unbounded();
        RegFile {
            registers: Default::default(),
            component_id,
            sim_manager,
            ack_sender,
            clock_sender: clock_channel.0,
            clock_receiver: clock_channel.1,
            rs1_idx_receiver,
            rs1_idx: Default::default(),
            rs1_idx_old: Default::default(),
            rs2_idx_receiver,
            rs2_idx: Default::default(),
            rs2_idx_old: Default::default(),
            rd_wr_receiver,
            rd_wr: Default::default(),
            rd_wr_old: Default::default(),
            rd_idx_receiver,
            rd_idx: Default::default(),
            rd_idx_old: Default::default(),
            rd_data_receiver,
            rd_data: Default::default(),
            rd_data_old: Default::default(),
            rs1_data_alu_mux1,
            rs1_data_cmp,
            rs2_data_alu_mux2,
            rs2_data_cmp_mux,
            rs2_data_data_out,
        }
    }

    fn init_impl(&mut self) {}

    fn reset_impl(&mut self) {
        self.registers.reset();
    }

    fn poll_impl(&mut self) {}

    fn on_clock(&mut self) {
        if self.rd_wr != Byte::zeros() {
            self.registers.write(self.rd_idx, self.rd_data);
        }
    }

    fn on_comb(&mut self) {
        send_word!(
            self,
            self.rs1_data_alu_mux1,
            self.registers.read(self.rs1_idx)
        );
        send_word!(self, self.rs1_data_cmp, self.registers.read(self.rs1_idx));
        send_word!(
            self,
            self.rs2_data_alu_mux2,
            self.registers.read(self.rs2_idx)
        );
        send_word!(
            self,
            self.rs2_data_cmp_mux,
            self.registers.read(self.rs2_idx)
        );
        send_word!(
            self,
            self.rs2_data_data_out,
            self.registers.read(self.rs2_idx)
        );
    }
}

impl Debug for RegFile {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "RegFile: {{rd_wr: {:?}, rd_idx: {:?}, rd_data: {:?}, rs1_idx: {:?}, rs2_idx: {:?}, data: {:?}}}", self.rd_wr, self.rd_idx, self.rd_data, self.rs1_idx, self.rs2_idx, self.registers)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend::util::blackhole::WordBlackhole;
    use crate::backend::util::event::ByteEvent;
    use crossbeam_channel::unbounded;
    use rsim_core::sim_dispatcher::SimDispatcher;
    use rsim_core::sim_manager::SimManager;
    use std::sync::{Arc, Mutex};
    use std::thread;

    #[test]
    // everything in one test cuz it takes a lot of code to set up :(
    fn test_simple() {
        let ack_channel = unbounded();

        let sim_manager = SimManager::new(ack_channel.1);

        let reg_file_clock_link = unbounded();
        let rs1_idx_link = unbounded();
        let rs2_idx_link = unbounded();
        let rd_wr_link = unbounded();
        let rd_idx_link = unbounded();
        let rd_data_link = unbounded();
        let rs1_data_alu_mux1_link = unbounded();
        let rs1_data_cmp_link = unbounded();
        let rs2_data_alu_mux2_link = unbounded();
        let rs2_data_cmp_mux_link = unbounded();
        let rs2_data_data_out_link = unbounded();

        let reg_file = Arc::new(Mutex::new(RegFile {
            registers: Registers::default(),
            component_id: 0,
            sim_manager: sim_manager.clone(),
            ack_sender: ack_channel.0.clone(),
            clock_sender: reg_file_clock_link.0,
            clock_receiver: reg_file_clock_link.1,
            rs1_idx_receiver: rs1_idx_link.1,
            rs1_idx: Bytes::unknown(),
            rs1_idx_old: Default::default(),
            rs2_idx_receiver: rs2_idx_link.1,
            rs2_idx: Bytes::unknown(),
            rs2_idx_old: Default::default(),
            rd_wr_receiver: rd_wr_link.1,
            rd_wr: Bytes::unknown(),
            rd_wr_old: Default::default(),
            rd_idx_receiver: rd_idx_link.1,
            rd_idx: Bytes::unknown(),
            rd_idx_old: Default::default(),
            rd_data_receiver: rd_data_link.1,
            rd_data: Bytes::unknown(),
            rd_data_old: Default::default(),
            rs1_data_alu_mux1: rs1_data_alu_mux1_link.0,
            rs1_data_cmp: rs1_data_cmp_link.0,
            rs2_data_alu_mux2: rs2_data_alu_mux2_link.0,
            rs2_data_cmp_mux: rs2_data_cmp_mux_link.0,
            rs2_data_data_out: rs2_data_data_out_link.0,
        }));

        sim_manager.register_do_not_end(0);

        let rs1_data_alu_mux1_blackhole = WordBlackhole::new(
            1,
            sim_manager.clone(),
            rs1_data_alu_mux1_link.1.clone(),
            ack_channel.0.clone(),
        );

        let rs1_data_cmp_blackhole = WordBlackhole::new(
            1,
            sim_manager.clone(),
            rs1_data_cmp_link.1.clone(),
            ack_channel.0.clone(),
        );

        let rs2_data_alu_mux2_blackhole = WordBlackhole::new(
            2,
            sim_manager.clone(),
            rs2_data_alu_mux2_link.1.clone(),
            ack_channel.0.clone(),
        );

        let rs2_data_cmp_mux_blackhole = WordBlackhole::new(
            2,
            sim_manager.clone(),
            rs2_data_cmp_mux_link.1.clone(),
            ack_channel.0.clone(),
        );

        let rs2_data_data_out_blackhole = WordBlackhole::new(
            2,
            sim_manager.clone(),
            rs2_data_data_out_link.1.clone(),
            ack_channel.0.clone(),
        );

        let sim_dispatchers = vec![
            SimDispatcher::new(Arc::downgrade(&sim_manager), vec![reg_file.clone()]),
            SimDispatcher::new(
                Arc::downgrade(&sim_manager),
                vec![
                    rs1_data_alu_mux1_blackhole.clone(),
                    rs1_data_cmp_blackhole.clone(),
                ],
            ),
            SimDispatcher::new(
                Arc::downgrade(&sim_manager),
                vec![
                    rs2_data_alu_mux2_blackhole.clone(),
                    rs2_data_cmp_mux_blackhole.clone(),
                    rs2_data_data_out_blackhole.clone(),
                ],
            ),
        ];
        sim_dispatchers.iter().for_each(|s| s.init());

        let mut thread_handlers = vec![];

        for sim_dispatcher in sim_dispatchers {
            thread_handlers.push(thread::spawn(move || sim_dispatcher.run()));
        }

        sim_manager.run_cycle();

        // test init
        for i in 0..32 {
            assert_eq!(
                reg_file.lock().unwrap().registers.read(Byte::from(i as u8)),
                Word::zeros()
            );
        }

        // test write
        for i in 0..32 {
            let wr = Byte::from(i as u8);
            let idx = Byte::from(i as u8);
            let data = Word::from(i as u32);
            sim_manager.proxy_event(
                Box::new(ByteEvent::new(
                    sim_manager.get_curr_cycle(),
                    wr,
                    sim_manager.request_new_event_id(),
                )),
                rd_wr_link.0.clone(),
            );
            sim_manager.proxy_event(
                Box::new(ByteEvent::new(
                    sim_manager.get_curr_cycle(),
                    idx,
                    sim_manager.request_new_event_id(),
                )),
                rd_idx_link.0.clone(),
            );
            sim_manager.proxy_event(
                Box::new(WordEvent::new(
                    sim_manager.get_curr_cycle(),
                    data,
                    sim_manager.request_new_event_id(),
                )),
                rd_data_link.0.clone(),
            );

            sim_manager.run_cycle();
            assert_eq!(reg_file.lock().unwrap().registers.read(idx), data);
        }

        // test read
        let curr_cycle = sim_manager.get_curr_cycle();
        for i in 0..16u8 {
            let rs1_idx = Byte::from(i * 2);
            let rs2_idx = Byte::from(i * 2 + 1);
            let rs1_data = Word::from((i * 2) as u32);
            let rs2_data = Word::from((i * 2 + 1) as u32);
            sim_manager.proxy_event(
                Box::new(ByteEvent::new(
                    sim_manager.get_curr_cycle(),
                    rs1_idx,
                    sim_manager.request_new_event_id(),
                )),
                rs1_idx_link.0.clone(),
            );
            sim_manager.proxy_event(
                Box::new(ByteEvent::new(
                    sim_manager.get_curr_cycle(),
                    rs2_idx,
                    sim_manager.request_new_event_id(),
                )),
                rs2_idx_link.0.clone(),
            );

            let _ = sim_manager.run_cycle_end();
            assert_eq!(sim_manager.get_curr_cycle(), curr_cycle);
            assert_eq!(
                rs1_data_alu_mux1_blackhole.lock().unwrap().get_input(),
                rs1_data
            );
            assert_eq!(rs1_data_cmp_blackhole.lock().unwrap().get_input(), rs1_data);
            assert_eq!(
                rs2_data_alu_mux2_blackhole.lock().unwrap().get_input(),
                rs2_data
            );
            assert_eq!(
                rs2_data_cmp_mux_blackhole.lock().unwrap().get_input(),
                rs2_data
            );
            assert_eq!(
                rs2_data_data_out_blackhole.lock().unwrap().get_input(),
                rs2_data
            );
        }

        // test write after read
        for i in 0..32 {
            let wr = Byte::from(i as u8);
            let idx = Byte::from(i as u8);
            let data = Word::from((i * 2) as u32);
            sim_manager.proxy_event(
                Box::new(ByteEvent::new(
                    sim_manager.get_curr_cycle(),
                    wr,
                    sim_manager.request_new_event_id(),
                )),
                rd_wr_link.0.clone(),
            );
            sim_manager.proxy_event(
                Box::new(ByteEvent::new(
                    sim_manager.get_curr_cycle(),
                    idx,
                    sim_manager.request_new_event_id(),
                )),
                rd_idx_link.0.clone(),
            );
            sim_manager.proxy_event(
                Box::new(WordEvent::new(
                    sim_manager.get_curr_cycle(),
                    data,
                    sim_manager.request_new_event_id(),
                )),
                rd_data_link.0.clone(),
            );

            sim_manager.run_cycle();
            assert_eq!(reg_file.lock().unwrap().registers.read(idx), data);
        }

        sim_manager.register_can_end(0);

        thread_handlers.into_iter().for_each(|h| {
            h.join().unwrap();
        });
    }
}

#[ComponentAttribute({
"port": {
    "input": [
        ["alu_out", "Word"],
        ["cmp_out", "Word"],
        ["u_imm", "Word"],
        ["mar", "Word"],
        ["mdr", "Word"],
        ["pc", "Word"],
        ["sel", "Byte"]
    ],
    "output": [
        ["out", "Word"]
    ]
}
})]
pub struct RegFileMux {}

impl RegFileMux {
    pub fn new(
        component_id: ComponentId,
        sim_manager: Arc<SimManager>,
        ack_sender: Sender<EventId>,
        alu_out_receiver: Input,
        cmp_out_receiver: Input,
        u_imm_receiver: Input,
        mar_receiver: Input,
        mdr_receiver: Input,
        pc_receiver: Input,
        sel_receiver: Input,
        out: Output,
    ) -> Self {
        RegFileMux {
            component_id,
            sim_manager,
            ack_sender,
            alu_out_receiver,
            alu_out: Default::default(),
            alu_out_old: Default::default(),
            cmp_out_receiver,
            cmp_out: Default::default(),
            cmp_out_old: Default::default(),
            u_imm_receiver,
            u_imm: Default::default(),
            u_imm_old: Default::default(),
            mar_receiver,
            mar: Default::default(),
            mar_old: Default::default(),
            mdr_receiver,
            mdr: Default::default(),
            mdr_old: Default::default(),
            pc_receiver,
            pc: Default::default(),
            pc_old: Default::default(),
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
        let mdr_idx = (Into::<Option<u32>>::into(self.mar).unwrap_or(0) & 0x3u32) as usize;
        let out = match self.sel.into() {
            Some(mux_sel::regfile::ALU_OUT) => self.alu_out,
            Some(mux_sel::regfile::BR_EN) => self.cmp_out,
            Some(mux_sel::regfile::U_IMM) => self.u_imm,
            Some(mux_sel::regfile::LW) => self.mdr,
            Some(mux_sel::regfile::PC_PLUS4) => self.pc + Word::from(4u32),
            Some(mux_sel::regfile::LB) => {
                let val = self.mdr[mdr_idx]
                    .map(|byte| Word::from(byte as u32))
                    .unwrap_or(Word::unknown());
                if !val.has_unknown() {
                    sign_extend(Into::<Option<u32>>::into(val).unwrap(), 7)
                } else {
                    Word::unknown()
                }
            }
            Some(mux_sel::regfile::LBU) => self.mdr[mdr_idx]
                .map(|byte| Word::from(byte as u32))
                .unwrap_or(Word::unknown()),
            Some(mux_sel::regfile::LH) => {
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
            Some(mux_sel::regfile::LHU) => self.mdr[mdr_idx]
                .map(|lsb| {
                    self.mdr[mdr_idx + 1]
                        .map(|msb| Word::from((((msb as u16) << 8) | lsb as u16) as u32))
                        .unwrap_or(Word::unknown())
                })
                .unwrap_or(Word::unknown()),
            _ => Word::unknown(),
        };

        send_word!(self, self.out, out);
    }
}

impl Debug for RegFileMux {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "RegFileMux: {{alu_out: {:?}, cmp_out: {:?}, u_imm: {:?}, mar: {:?} mdr: {:?} pc: {:?} sel: {:?}}}", self.alu_out, self.cmp_out, self.u_imm, self.mar, self.mdr, self.pc, self.sel)
    }
}
