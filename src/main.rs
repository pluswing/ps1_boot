use cpu::Cpu;

mod cpu;
mod bios;

fn main() {
  let cpu = Cpu::new();
  println!("Hello, world!");
}
