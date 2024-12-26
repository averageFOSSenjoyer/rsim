use crate::mmio_ctl::MMIOCtl;
use crate::mmio_driver::MMIODriver;
use crate::types::Byte;
use crate::types::Word;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Debug;

#[derive(Debug)]
pub struct MemCtl {
    backend_mem: BTreeMap<Word, Byte>,
    mmio_addr: BTreeSet<Word>,
    mmio_ctl: MMIOCtl,
    mem_access_latency: u128,
    mem_wait_left: u128,
}

impl MemCtl {
    pub fn is_resp(&self) -> bool {
        self.mem_wait_left == 0
    }

    pub fn req_access(&mut self) {
        if self.mem_wait_left == 0 {
            self.mem_wait_left = self.mem_access_latency;
        }
    }

    pub fn tick(&mut self) {
        self.mem_wait_left = (self.mem_wait_left - 1).min(0);
    }

    pub fn read(&self, addr: &Word, rmask: Byte) -> Word {
        let mut ret = Word::unknown();
        if let Some(rmask) = Into::<Option<u8>>::into(rmask) {
            for i in 0..4 {
                if rmask >> i & 0x1 == 0x1 {
                    let addr_idx = *addr + Word::from(i as u32);
                    ret[i] = if self.mmio_addr.contains(&addr_idx) {
                        self.mmio_ctl.read(&addr_idx).into()
                    } else if self.backend_mem.contains_key(&addr_idx) {
                        self.backend_mem[&addr_idx].into()
                    } else {
                        None
                    }
                }
            }
        }

        ret
    }

    // pub fn read(&self, addr: &Word) -> Word {
    //     let mut ret = Word::unknown();
    //     for i in 0..4 {
    //         let addr_idx = *addr + Word::from(i as u32);
    //         ret[i] = if self.mmio_addr.contains(&addr_idx) {
    //             self.mmio_ctl.read(&addr_idx).into()
    //         } else if self.backend_mem.contains_key(&addr_idx) {
    //             self.backend_mem[&addr_idx].into()
    //         } else {
    //             None
    //         }
    //     }
    //
    //     ret
    // }

    pub fn write(&mut self, addr: &Word, data: Word, wmask: Byte) {
        if let Some(wmask) = Into::<Option<u8>>::into(wmask) {
            for i in 0..4 {
                if wmask >> i & 0x1 == 0x1 {
                    let addr_idx = *addr + Word::from(i as u32);
                    let data = data[i]
                        .map(|byte| Byte::from(byte))
                        .unwrap_or(Byte::unknown());
                    if self.mmio_addr.contains(&addr_idx) {
                        self.mmio_ctl.write(&addr_idx, data);
                    } else {
                        self.backend_mem.insert(addr_idx, data);
                    }
                }
            }
        }
    }

    pub fn insert_mmio_driver(&mut self, addr: &Word, driver: Box<dyn MMIODriver>) {
        self.mmio_addr.insert(*addr);
        self.mmio_ctl.insert_mmio_driver(addr, driver);
    }

    pub fn load_bin(&mut self, data: &Vec<u8>, addr: Word) {
        for i in 0..data.len() as u32 {
            self.backend_mem
                .insert(addr + Word::from(i), data[i as usize].into());
        }
    }
}

impl Default for MemCtl {
    fn default() -> Self {
        Self {
            backend_mem: Default::default(),
            mmio_addr: Default::default(),
            mmio_ctl: MMIOCtl::default(),
            mem_access_latency: 2,
            mem_wait_left: 0,
        }
    }
}

// impl Debug for MemCtl {
//     fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
//         write!(f, "mem_wait_left: {}", self.mem_wait_left)
//     }
// }
