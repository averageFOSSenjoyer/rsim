use crate::core::Core;
use crate::types::Word;
use std::fs;

pub mod byte;
mod control;
pub mod core;
mod ir;
pub mod mem_ctl;
mod mmio_ctl;
mod mmio_driver;
mod regfile;
pub mod types;
mod util;

fn main() {
    let mut core = Core::default();

    let data = fs::read("/home/frank/dev/sim_poc/unicore/coremark_rv32i.bin").unwrap();
    core.load_bin(&data, Word::from(0x40000000u32));

    while !core.should_halt() {
        core.next_cycle();
        // println!("{:?}", core);
    }
    // for i in 0..9000 {
    //     core.next_cycle();
    //     // println!("{:?}", core);
    // }
}
