use crate::backend::util::byte::Shra;
use crate::backend::util::event::WordEvent;
use crate::backend::util::types::Word;
use crate::backend::util::types::*;
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
        ["out_pc_mux", "Word"],
        ["out_mar_mux", "Word"],
        ["out_regfile_mux", "Word"]
    ]
}
})]
pub struct Alu {}

impl Alu {
    pub fn new(
        component_id: ComponentId,
        sim_manager: Arc<SimManager>,
        ack_sender: Sender<EventId>,
        a_receiver: Input,
        b_receiver: Input,
        op_receiver: Input,
        out_pc_mux: Output,
        out_mar_mux: Output,
        out_regfile_mux: Output,
    ) -> Self {
        Alu {
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
            out_pc_mux,
            out_mar_mux,
            out_regfile_mux,
        }
    }
    fn init_impl(&mut self) {}

    fn reset_impl(&mut self) {}

    fn poll_impl(&mut self) {}

    fn on_comb(&mut self) {
        let out = match Into::<Option<u8>>::into(self.op) {
            Some(alu_op::ADD) => self.a + self.b,
            Some(alu_op::SLL) => self.a << (self.b & Word::from(0x1Fu32)),
            Some(alu_op::SRA) => self.a.shra(self.b & Word::from(0x1Fu32)),
            Some(alu_op::SUB) => self.a - self.b,
            Some(alu_op::XOR) => self.a ^ self.b,
            Some(alu_op::SRL) => self.a >> (self.b & Word::from(0x1Fu32)),
            Some(alu_op::OR) => self.a | self.b,
            Some(alu_op::AND) => self.a & self.b,
            _ => Word::unknown(),
        };

        send_word!(self, self.out_pc_mux, out);
        send_word!(self, self.out_mar_mux, out);
        send_word!(self, self.out_regfile_mux, out);
    }
}

impl Debug for Alu {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Alu: {{a: {:?}, b:{:?}, op: {:?} }}",
            self.a, self.b, self.op
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend::util::blackhole::WordBlackhole;
    use crate::backend::util::event::ByteEvent;
    use crate::backend::util::event::WordEvent;
    use crossbeam_channel::unbounded;
    use rand::random;
    use rsim_core::sim_dispatcher::SimDispatcher;
    use rsim_core::sim_manager::SimManager;
    use std::sync::Arc;
    use std::sync::Mutex;
    use std::thread;

    #[test]
    fn test_simple() {
        let ack_channel = unbounded();

        let sim_manager = SimManager::new(ack_channel.1);
        let a_link = unbounded();
        let b_link = unbounded();
        let op_link = unbounded();
        let out_pc_mux_link = unbounded();
        let out_mar_mux_link = unbounded();
        let out_regfile_mux_link = unbounded();

        let alu = Arc::new(Mutex::new(Alu::new(
            0,
            sim_manager.clone(),
            ack_channel.0.clone(),
            a_link.1.clone(),
            b_link.1.clone(),
            op_link.1.clone(),
            out_pc_mux_link.0.clone(),
            out_mar_mux_link.0.clone(),
            out_regfile_mux_link.0.clone(),
        )));

        sim_manager.register_do_not_end(0);

        let out_pc_mux_blackhole = WordBlackhole::new(
            1,
            sim_manager.clone(),
            out_pc_mux_link.1.clone(),
            ack_channel.0.clone(),
        );

        let out_mar_mux_blackhole = WordBlackhole::new(
            1,
            sim_manager.clone(),
            out_mar_mux_link.1.clone(),
            ack_channel.0.clone(),
        );

        let out_regfile_mux_blackhole = WordBlackhole::new(
            1,
            sim_manager.clone(),
            out_regfile_mux_link.1.clone(),
            ack_channel.0.clone(),
        );

        let sim_dispatchers = vec![
            SimDispatcher::new(Arc::downgrade(&sim_manager), vec![alu.clone()]),
            SimDispatcher::new(
                Arc::downgrade(&sim_manager),
                vec![out_pc_mux_blackhole.clone()],
            ),
            SimDispatcher::new(
                Arc::downgrade(&sim_manager),
                vec![out_mar_mux_blackhole.clone()],
            ),
            SimDispatcher::new(
                Arc::downgrade(&sim_manager),
                vec![out_regfile_mux_blackhole.clone()],
            ),
        ];

        let mut thread_handlers = vec![];

        for sim_dispatcher in sim_dispatchers {
            thread_handlers.push(thread::spawn(move || sim_dispatcher.run()));
        }

        for i in 0..16u8 {
            let a_u32 = random::<u32>();
            let b_u32 = random::<u32>();
            let op_u8 = random::<u8>() % (alu_op::AND + 1);
            let a = Word::from(a_u32);
            let b = Word::from(b_u32);
            let op = Byte::from(op_u8);

            sim_manager.proxy_event(
                Box::new(WordEvent::new(
                    sim_manager.get_curr_cycle(),
                    a,
                    sim_manager.request_new_event_id(),
                )),
                a_link.0.clone(),
            );
            sim_manager.proxy_event(
                Box::new(WordEvent::new(
                    sim_manager.get_curr_cycle(),
                    b,
                    sim_manager.request_new_event_id(),
                )),
                b_link.0.clone(),
            );
            sim_manager.proxy_event(
                Box::new(ByteEvent::new(
                    sim_manager.get_curr_cycle(),
                    op,
                    sim_manager.request_new_event_id(),
                )),
                op_link.0.clone(),
            );

            let _ = sim_manager.run_cycle_end();
            let expected_result = match op_u8 {
                alu_op::ADD => Word::from(a_u32 + b_u32),
                alu_op::SLL => Word::from(a_u32 << b_u32),
                alu_op::SRA => Word::from(((a_u32 as i32) >> (b_u32 as i32)) as u32),
                alu_op::SUB => Word::from(a_u32 - b_u32),
                alu_op::XOR => Word::from(a_u32 ^ b_u32),
                alu_op::SRL => Word::from(a_u32 >> b_u32),
                alu_op::OR => Word::from(a_u32 | b_u32),
                alu_op::AND => Word::from(a_u32 & b_u32),
                _ => Word::unknown(),
            };

            assert_eq!(
                out_pc_mux_blackhole.lock().unwrap().get_input(),
                expected_result
            );
            assert_eq!(
                out_mar_mux_blackhole.lock().unwrap().get_input(),
                expected_result
            );
            assert_eq!(
                out_regfile_mux_blackhole.lock().unwrap().get_input(),
                expected_result
            );
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
        ["rs1", "Word"],
        ["pc", "Word"],
        ["sel", "Byte"]
    ],
    "output": [
        ["out", "Word"]
    ]
}
})]
pub struct AluMux1 {}

impl AluMux1 {
    pub fn new(
        component_id: ComponentId,
        sim_manager: Arc<SimManager>,
        ack_sender: Sender<EventId>,
        rs1_receiver: Input,
        pc_receiver: Input,
        sel_receiver: Input,
        out: Output,
    ) -> Self {
        AluMux1 {
            component_id,
            sim_manager,
            ack_sender,
            rs1_receiver,
            rs1: Default::default(),
            rs1_old: Default::default(),
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
        let out = match self.sel.into() {
            Some(mux_sel::alu1::RS1_OUT) => self.rs1,
            Some(mux_sel::alu1::PC_OUT) => self.pc,
            _ => Word::unknown(),
        };

        send_word!(self, self.out, out);
    }
}

#[ComponentAttribute({
"port": {
    "input": [
        ["i_imm", "Word"],
        ["u_imm", "Word"],
        ["b_imm", "Word"],
        ["s_imm", "Word"],
        ["j_imm", "Word"],
        ["rs2", "Word"],
        ["sel", "Byte"]
    ],
    "output": [
        ["out", "Word"]
    ]
}
})]
pub struct AluMux2 {}

impl AluMux2 {
    pub fn new(
        component_id: ComponentId,
        sim_manager: Arc<SimManager>,
        ack_sender: Sender<EventId>,
        i_imm_receiver: Input,
        u_imm_receiver: Input,
        b_imm_receiver: Input,
        s_imm_receiver: Input,
        j_imm_receiver: Input,
        rs2_receiver: Input,
        sel_receiver: Input,
        out: Output,
    ) -> Self {
        AluMux2 {
            component_id,
            sim_manager,
            ack_sender,
            i_imm_receiver,
            i_imm: Default::default(),
            i_imm_old: Default::default(),
            u_imm_receiver,
            u_imm: Default::default(),
            u_imm_old: Default::default(),
            b_imm_receiver,
            b_imm: Default::default(),
            b_imm_old: Default::default(),
            s_imm_receiver,
            s_imm: Default::default(),
            s_imm_old: Default::default(),
            j_imm_receiver,
            j_imm: Default::default(),
            j_imm_old: Default::default(),
            rs2_receiver,
            rs2: Default::default(),
            rs2_old: Default::default(),
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
            Some(mux_sel::alu2::I_IMM) => self.i_imm,
            Some(mux_sel::alu2::U_IMM) => self.u_imm,
            Some(mux_sel::alu2::B_IMM) => self.b_imm,
            Some(mux_sel::alu2::S_IMM) => self.s_imm,
            Some(mux_sel::alu2::J_IMM) => self.j_imm,
            Some(mux_sel::alu2::RS2_OUT) => self.rs2,
            _ => Word::unknown(),
        };

        send_word!(self, self.out, out);
    }
}
