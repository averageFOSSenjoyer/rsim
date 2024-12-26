use crate::mmio_driver::MMIODriver;
use crate::types::Byte;
use crate::types::Word;
use std::collections::BTreeMap;
use std::fmt::{Debug, Formatter};

pub struct MMIOCtl {
    mmio_drivers: BTreeMap<Word, Box<dyn MMIODriver>>,
}

impl MMIOCtl {
    pub fn read(&self, addr: &Word) -> Byte {
        if self.mmio_drivers.contains_key(&addr) {
            self.mmio_drivers[&addr].read()
        } else {
            Byte::unknown()
        }
    }

    pub fn write(&self, addr: &Word, data: Byte) {
        if self.mmio_drivers.contains_key(&addr) {
            self.mmio_drivers[&addr].write(data);
        }
    }

    pub fn insert_mmio_driver(&mut self, addr: &Word, driver: Box<dyn MMIODriver>) {
        self.mmio_drivers.insert(*addr, driver);
    }
}

impl Default for MMIOCtl {
    fn default() -> Self {
        Self {
            mmio_drivers: BTreeMap::new(),
        }
    }
}

impl Debug for MMIOCtl {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "")
    }
}