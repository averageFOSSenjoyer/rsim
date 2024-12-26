use crate::types::Byte;
use crate::types::Word;
use crate::util::sign_extend;

#[derive(Debug)]
pub struct IR {
    pub data: Word,
    pub funct3: Byte,
    pub funct7: Byte,
    pub opcode: Byte,
    pub i_imm: Word,
    pub s_imm: Word,
    pub b_imm: Word,
    pub u_imm: Word,
    pub j_imm: Word,
    pub rs1: Byte,
    pub rs2: Byte,
    pub rd: Byte,
}

impl IR {
    pub fn write(&mut self, data: Word) {
        self.data = data;
        self.update();
    }

    fn update(&mut self) {
        if let Some(inst) = Into::<Option<u32>>::into(self.data) {
            self.funct3 = Byte::from(((inst >> 12) & 0b111) as u8);
            self.funct7 = Byte::from(((inst >> 25) & 0b1111111) as u8);
            self.opcode = Byte::from((inst & 0b1111111) as u8);
            self.i_imm = sign_extend(inst >> 20, 11);
            self.s_imm = sign_extend(
                (((inst >> 25) & 0b1111111) << 5) | ((inst >> 7) & 0b11111),
                11,
            );
            self.b_imm = sign_extend(
                (((inst >> 31) & 0b1) << 12)
                    | (((inst >> 7) & 0b1) << 11)
                    | (((inst >> 25) & 0b111111) << 5)
                    | (((inst >> 8) & 0b1111) << 1)
                    | 0b0,
                12,
            );
            self.u_imm = Word::from(((inst >> 12) & 0xFFFFF) << 12);
            self.j_imm = sign_extend(
                (((inst >> 31) & 0b1) << 20)
                    | (((inst >> 12) & 0xFF) << 12)
                    | (((inst >> 20) & 0b1) << 11)
                    | (((inst >> 21) & 0x3FF) << 1)
                    | 0b0,
                20,
            );
            self.rs1 = Byte::from(((inst >> 15) & 0x1F) as u8);
            self.rs2 = Byte::from(((inst >> 20) & 0x1F) as u8);
            self.rd = Byte::from(((inst >> 7) & 0x1F) as u8);
        }
    }
}

impl Default for IR {
    fn default() -> Self {
        Self {
            data: Default::default(),
            funct3: Default::default(),
            funct7: Default::default(),
            opcode: Default::default(),
            i_imm: Default::default(),
            s_imm: Default::default(),
            b_imm: Default::default(),
            u_imm: Default::default(),
            j_imm: Default::default(),
            rs1: Default::default(),
            rs2: Default::default(),
            rd: Default::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    pub fn test_sign_extend() {
        let val = -128 << 8;
        assert_eq!(
            Word::from(-128i32 as u32),
            sign_extend((val >> 8) as u32, 8)
        );
    }
}
