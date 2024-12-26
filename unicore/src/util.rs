use crate::types::Word;

pub fn sign_extend(val: u32, upper_idx: u8) -> Word {
    let msb = (val >> upper_idx) & 0x1;
    if msb == 0x1 {
        let mut tmp = val;
        for i in upper_idx + 1..32 {
            tmp |= 0x1 << i;
        }
        Word::from(tmp)
    } else {
        Word::from(val & (0xFFFFFFFF >> (32 - (upper_idx + 1))))
    }
}
