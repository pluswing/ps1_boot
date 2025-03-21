use std::path::Path;

use bios::Bios;
use cpu::Cpu;
use interconnect::Interconnect;

mod cpu;
mod bios;
mod interconnect;
mod ram;
mod dma;
mod channel;

fn main() {
  let bios = Bios::new(&Path::new("bios/BIOS.ROM")).unwrap();
  let inter = Interconnect::new(bios);
  let mut cpu = Cpu::new(inter);
  loop {
    cpu.run_next_instruction();
  }
}
