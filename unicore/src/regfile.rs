use crate::types::Byte;
use crate::types::Word;

#[derive(Debug)]
pub struct RegFile {
    registers: [Word; 32],
}

impl RegFile {
    pub fn read(&self, addr: Byte) -> Word {
        if addr.is_zero() {
            Word::zeros()
        } else if addr.is_something_nonzero() {
            self.registers[Into::<Option<u32>>::into(addr).unwrap() as usize]
        } else {
            Word::unknown()
        }
    }

    pub fn write(&mut self, addr: Byte, val: Word) {
        if addr.is_something_nonzero() {
            self.registers[Into::<Option<u32>>::into(addr).unwrap() as usize] = val;
        }
    }
}

impl Default for RegFile {
    // fn default() -> Self {
    //     let mut regs = [Word::unknown(); 32];
    //     regs[0] = Word::zeros();
    //     Self {
    //         registers: regs
    //     }
    // }
    fn default() -> Self {
        let regs = [Word::zeros(); 32];
        Self { registers: regs }
    }
}
