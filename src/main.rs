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
  let video_subsystem = sdl_context.video().unwrap();
  let audio_subsystem = sdl_context.audio().unwrap();

  let gpu = Gpu::new(video_subsystem);
  let spu = Spu::new(audio_subsystem);
  let inter = Interconnect::new(bios, gpu, spu);
  let mut cpu = Cpu::new(inter);
  let mut event_pump = sdl_context.event_pump().unwrap();

  let mut counter = 0;
  loop {
    cpu.run_next_instruction();
    counter = counter + 1;
    if counter >= 768 {
      counter = 0;
      cpu.inter.spu.clock();
    }
    if cpu.inter.gpu.frame_updated {
      cpu.inter.gpu.frame_updated = false;
      for event in event_pump.poll_iter() {
        match event {
          sdl2::event::Event::Quit {..} => panic!("exit!"),
          _ => {},
        }
      }
    }
  }
}
