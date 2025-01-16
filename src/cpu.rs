pub struct Cpu {
  pc: u32,
}

impl Cpu {
  pub fn new() -> Self {
    Self {
      pc: 0xBFC0_0000,
    }
  }
  pub fn run_next_instruction(&mut self) {
    let pc = self.pc;
    let instruction = self.load32(pc);
    self.pc = pc.wrapping_add(4);
    self.decode_and_execute(instruction);
  }
}
