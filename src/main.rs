use core::time;
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

  let spu_interval = 1_000_000_000 / 44100;
  let mut spu_now = Instant::now();
  let mut nanos: u128 = 0;

  let mut spu_counter = 0;
  let mut gpu_counter = 0;

  let gpu_interval = 1_000_000_000 / 60;
  let mut gpu_now = Instant::now();

  let mut now2 = Instant::now();

  loop {
    cpu.run_next_instruction();

    let n = spu_now.elapsed().as_nanos();
    if n > spu_interval * 20 {
      nanos += n;
      while nanos >= spu_interval {
        nanos -= spu_interval;
        cpu.inter.spu.clock();
        spu_counter += 1;
      }
      spu_now = Instant::now();
    }

    if cpu.inter.gpu.frame_updated {
      cpu.inter.gpu.frame_updated = false;
      for event in event_pump.poll_iter() {
        match event {
          sdl2::event::Event::Quit {..} => panic!("exit!"),
          _ => {},
        }
      }
      if gpu_now.elapsed().as_nanos() < gpu_interval {
        std::thread::sleep(time::Duration::from_nanos((gpu_interval - gpu_now.elapsed().as_nanos()) as u64));
      }
      gpu_now = Instant::now();
      gpu_counter += 1;
    }

    if now2.elapsed().as_nanos() >= 1_000_000_000 {
      println!("SPU: {}, GPU: {}", spu_counter, gpu_counter);
      spu_counter = 0;
      gpu_counter = 0;
      now2 = Instant::now();
    }
  }
}
