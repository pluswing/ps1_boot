use std::path::Path;

use bios::Bios;
use cpu::Cpu;
use gpu::Gpu;
use interconnect::Interconnect;
use spu::Spu;

mod cpu;
mod bios;
mod interconnect;
mod ram;
mod dma;
mod channel;
mod gpu;
mod renderer;
mod spu;

fn main() {
  let bios = Bios::new(&Path::new("bios/BIOS.ROM")).unwrap();

  let sdl_context = sdl2::init().unwrap();
  let gpu = Gpu::new(sdl_context);
  let spu = Spu::new(sdl_context);
  let inter = Interconnect::new(bios, gpu, spu);
  let mut cpu = Cpu::new(inter);
  loop {
    cpu.run_next_instruction();
  }
}
