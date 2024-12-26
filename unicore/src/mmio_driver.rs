use crate::types::Byte;

pub trait MMIODriver {
    fn init(&self);
    fn read(&self) -> Byte;
    fn write(&self, byte: Byte);
}
