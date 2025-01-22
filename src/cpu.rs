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
    self.decode_and_execute(Instruction(instruction));
  }

  fn load32(&self, addr: u32) -> u32 {
    self.inter.load32(addr)
  }

  fn decode_and_execute(&mut self, instruction: Instruction) {
    match instruction.function() {
      0b001111 => self.op_lui(instruction),
      _ => panic!("Unhandled instruction {:08X}", instruction.0),
    }
  }

  fn op_lui(&mut self, instruction: Instruction) {
    let i = instruction.imm();
    let t = instruction.t();
    panic!("what now?");
  }
}

struct Instruction(u32);

impl Instruction {
  fn function(&self) -> u32 {
    let Instruction(op) = self;
    op >> 26
  }

  fn t(&self) -> u32 {
    let Instruction(op) = self;
    (op >> 16) & 0x1F
  }

  fn imm(&self) -> u32 {
    let Instruction(op) = self;
    op & 0xFFFF
  }
}
