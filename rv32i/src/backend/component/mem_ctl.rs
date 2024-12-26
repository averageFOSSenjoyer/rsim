use crate::backend::util::event::ByteEvent;
use crate::backend::util::event::WordEvent;
use crate::backend::util::types::{Byte, Word};
use crate::send_byte;
use crate::send_word;
use crossbeam_channel::{unbounded, Sender};
use rsim_core::ack;
use rsim_core::component::Component;
use rsim_core::event::get_inner;
use rsim_core::sim_manager::SimManager;
use rsim_core::task::Task;
use rsim_core::types::ComponentId;
use rsim_core::types::EventId;
use rsim_core::types::Input;
use rsim_core::types::Output;
use rsim_core::{enq, send};
use rsim_macro::ComponentAttribute;
use std::collections::{BTreeMap, HashSet};
use std::fmt::{Debug, Formatter};
use std::sync::Arc;

#[ComponentAttribute({
"port": {
    "input": [
        ["cpu_addr", "Word"],

        ["cpu_wdata", "Word"],
        ["cpu_read_en", "Byte"],
        ["cpu_rmask", "Byte"],
        ["cpu_write_en", "Byte"],
        ["cpu_wmask", "Byte"]
    ],
    "output": [
        ["cpu_rdata", "Word"],
        ["cpu_resp", "Byte"]
    ],
    "clock": true
}
})]
#[allow(dead_code)]
pub struct MemCtl {
    pub backend_mem: BTreeMap<Word, Byte>,
    mmio_addr: HashSet<Word>,
    is_busy: bool,
}

impl MemCtl {
    pub fn new(
        component_id: ComponentId,
        sim_manager: Arc<SimManager>,
        ack_sender: Sender<EventId>,
        cpu_addr_receiver: Input,
        cpu_wdata_receiver: Input,
        cpu_read_en_receiver: Input,
        cpu_rmask_receiver: Input,
        cpu_write_en_receiver: Input,
        cpu_wmask_receiver: Input,
        cpu_rdata: Output,
        cpu_resp: Output,
    ) -> Self {
        let clock_channel = unbounded();

        MemCtl {
            backend_mem: Default::default(),
            mmio_addr: Default::default(),
            is_busy: false,
            component_id,
            sim_manager,
            ack_sender,
            clock_sender: clock_channel.0,
            clock_receiver: clock_channel.1,
            cpu_addr_receiver,
            cpu_addr: Default::default(),
            cpu_addr_old: Default::default(),
            cpu_wdata_receiver,
            cpu_wdata: Default::default(),
            cpu_wdata_old: Default::default(),
            cpu_read_en_receiver,
            cpu_read_en: Default::default(),
            cpu_read_en_old: Default::default(),
            cpu_rmask_receiver,
            cpu_rmask: Default::default(),
            cpu_rmask_old: Default::default(),
            cpu_write_en_receiver,
            cpu_write_en: Default::default(),
            cpu_write_en_old: Default::default(),
            cpu_wmask_receiver,
            cpu_wmask: Default::default(),
            cpu_wmask_old: Default::default(),
            cpu_rdata,
            cpu_resp,
        }
    }

    fn init_impl(&mut self) {}

    fn reset_impl(&mut self) {}

    fn poll_impl(&mut self) {}

    fn on_clock(&mut self) {
        //can recv request
        if !self.is_busy {
            // a r/w request came in
            if self.cpu_write_en.is_something_nonzero() {
                if let Some(wmask) = Into::<Option<u8>>::into(self.cpu_wmask) {
                    for i in 0..4 {
                        if wmask >> i & 0x1 == 0x1 {
                            let addr_idx = self.cpu_addr + Word::from(i as u32);
                            let data = self.cpu_wdata[i].map(Byte::from).unwrap_or(Byte::unknown());
                            self.backend_mem.insert(addr_idx, data);
                        }
                    }
                }
                send_byte!(self, self.cpu_resp, Byte::from(1u8));
            } else if self.cpu_read_en.is_something_nonzero() {
                let mut ret = Word::unknown();
                if let Some(rmask) = Into::<Option<u8>>::into(self.cpu_rmask) {
                    for i in 0..4 {
                        if rmask >> i & 0x1 == 0x1 {
                            let addr_idx = self.cpu_addr + Word::from(i as u32);
                            ret[i] = if self.backend_mem.contains_key(&addr_idx) {
                                self.backend_mem[&addr_idx].into()
                            } else {
                                None
                            }
                        }
                    }
                }

                send_word!(self, self.cpu_rdata, ret);
                send_byte!(self, self.cpu_resp, Byte::from(1u8));
            } else {
                send_byte!(self, self.cpu_resp, Byte::from(0u8));
            }
        }
    }

    fn on_comb(&mut self) {}

    pub fn load_bin(&mut self, data: &[u8], addr: Word) {
        for i in 0..data.len() as u32 {
            self.backend_mem
                .insert(addr + Word::from(i), data[i as usize].into());
        }
    }
}

impl Debug for MemCtl {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "MemCtl: {{cpu_addr: {:?}, wmask: {:?}}}",
            self.cpu_addr, self.cpu_wmask
        )
    }
}

// fn on_clock(&mut self) {
//     let curr_cycle = self.sim_manager.get_curr_cycle();
//
//     // can recv request
//     if !self.is_busy {
//         // a r/w request came in
//         if self.cpu_write_en.is_something_nonzero() || self.cpu_read_en.is_something_nonzero() {
//             let (en_port, wdata_port) = match (
//                 self.mmio_addr.contains(&self.cpu_addr), // is this a mmio peripheral?
//                 self.cpu_read_en.is_something_nonzero(),
//                 self.cpu_write_en.is_something_nonzero(),
//             ) {
//                 (true, true, _) => (self.mmio_read_en.clone(), self.mmio_wdata.clone()),
//                 (true, _, true) => (self.mmio_write_en.clone(), self.mmio_wdata.clone()),
//                 (false, true, _) => (self.mem_read_en.clone(), self.mem_wdata.clone()),
//                 (false, _, true) => (self.mem_write_en.clone(), self.mem_wdata.clone()),
//                 _ => {
//                     panic!("This should never get called")
//                 }
//             };
//
//             // forward enable to either backend or mmio_ctl
//             send_byte!(self, en_port, Byte::from(1u8));
//
//             // also forward wdata if any
//             if self.cpu_write_en.is_something_nonzero() {
//                 let wdata_event = WordEvent::new(
//                     curr_cycle,
//                     self.cpu_wdata,
//                     self.sim_manager.request_new_event_id(),
//                 );
//                 send!(self, wdata_port, wdata_event);
//             }
//             self.is_busy = true;
//         }
//     }
//
//     // is processing
//     if self.is_busy {
//         // resp came in
//         if self.mmio_resp.is_something_nonzero() || self.mem_resp.is_something_nonzero() {
//             // clear the enables
//             send_byte!(self, self.mem_read_en, Byte::from(0u8));
//             send_byte!(self, self.mem_write_en, Byte::from(0u8));
//             send_byte!(self, self.mmio_read_en, Byte::from(0u8));
//             send_byte!(self, self.mmio_write_en, Byte::from(0u8));
//
//             self.is_busy = false;
//         }
//     }
// }
//
// fn on_comb(&mut self) {
//     send_word!(self, self.mem_wdata, self.cpu_wdata);
//     send_word!(self, self.mmio_wdata, self.cpu_wdata);
//     send_byte!(self, self.mem_wmask, self.cpu_wmask);
//     send_byte!(self, self.mem_rmask, self.cpu_rmask);
//     send_byte!(self, self.mmio_wmask, self.cpu_wmask);
//     send_byte!(self, self.mmio_rmask, self.cpu_rmask);
//
//     // a request finished processing
//     if self.mmio_resp.is_something_nonzero() || self.mem_resp.is_something_nonzero() {
//         // it was a read request, need to tell cpu
//         if self.cpu_read_en.is_something_nonzero() {
//             let rdata = if self.mmio_resp.is_something_nonzero() {
//                 self.mmio_rdata
//             } else {
//                 self.mem_rdata
//             };
//             send_word!(self, self.cpu_rdata, rdata);
//         }
//         // tell cpu we finished processing
//         send_byte!(self, self.cpu_resp, Byte::from(1u8));
//     }
// }
