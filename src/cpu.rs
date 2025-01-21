use crate::interconnect::Interconnect;

pub struct Cpu {
  pc: u32,
  inter: Interconnect,
}

impl Cpu {
  pub fn new(inter: Interconnect) -> Self {
    Self {
      pc: 0xBFC0_0000,
      inter
    }
  }
  pub fn run_next_instruction(&mut self) {
    let pc = self.pc;
    let instruction = self.load32(pc);
    self.pc = pc.wrapping_add(4);
    self.decode_and_execute(instruction);
  }

  fn load32(&self, addr: u32) -> u32 {
    self.inter.load32(addr)
  }

  fn decode_and_execute(&mut self, instruction: u32) {
    panic!("Unhandled instruction {:08X}", instruction);
  }
}
