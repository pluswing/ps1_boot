use cpu::Cpu;

mod cpu;
mod bios;
mod interconnect;
mod map;

fn main() {
  let cpu = Cpu::new();
  println!("Hello, world!");
}
