use std::{path::Path, time::Instant};

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

  let interval = 1_000_000_000 / 44100;
  let mut now = Instant::now();
  let mut nanos: u128 = 0;

  let mut now2 = Instant::now();
  let mut counter = 0;
  loop {
    cpu.run_next_instruction();

    let n = now.elapsed().as_nanos();
    if n > interval * 20 {
      nanos += n;
      while nanos >= interval {
        nanos -= interval;
        cpu.inter.spu.clock();
        counter += 1;
      }
      now = Instant::now();
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

    if now2.elapsed().as_nanos() >= 1_000_000_000 {
      println!("SPU CLOCK: {}", counter);
      counter = 0;
      now2 = Instant::now();
    }
  }
}
