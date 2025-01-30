use crate::interconnect::Interconnect;

pub struct Cpu {
  pc: u32,
  regs: [u32; 32],
  inter: Interconnect,
}

impl Cpu {
  pub fn new(inter: Interconnect) -> Self {
    let mut regs = [0xDEAD_BEEF; 32];
    regs[0] = 0;
    Self {
      pc: 0xBFC0_0000,
      regs,
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

  fn store32(&mut self, addr: u32, val: u32) {
    self.inter.store32(addr, val)
  }

  fn decode_and_execute(&mut self, instruction: Instruction) {
    match instruction.function() {
      0x000000 => match instruction.subfunction() {
        0b000000 => self.op_sll(instruction),
        _ => panic!("Unhandled instrcuntion {:08X}", instruction.0),
      },
      0b001111 => self.op_lui(instruction),
      0b001101 => self.op_ori(instruction),
      0b101011 => self.op_sw(instruction),
      _ => panic!("Unhandled instruction {:08X}", instruction.0),
    }
  }

  fn reg(&self, index: u32) -> u32 {
    self.regs[index as usize]
  }

  fn set_reg(&mut self, index: u32, val: u32) {
    self.regs[index as usize] = val;
    self.regs[0] = 0;
  }

  fn op_lui(&mut self, instruction: Instruction) {
    let i = instruction.imm();
    let t = instruction.t();
    let v = i << 16;
    self.set_reg(t, v);
  }

  fn op_ori(&mut self, instruction: Instruction) {
    let i = instruction.imm();
    let t = instruction.t();
    let s = instruction.s();
    let v = self.reg(s) | i;
    self.set_reg(t, v);
  }

  fn op_sw(&mut self, instruction: Instruction) {
    let i = instruction.imm_se();
    let t = instruction.t();
    let s = instruction.s();

    let addr = self.reg(s).wrapping_add(i);
    let v = self.reg(t);
    self.store32(addr, v);
  }

  fn op_sll(&mut self, instruction: Instruction) {
    let i = instruction.shift();
    let t = instruction.t();
    let d = instruction.d();

    let v = self.reg(t) << i;
    self.set_reg(d, v);
  }

  fn op_addiu(&mut self, instruction: Instruction) {
    let i = instruction.imm_se();
    let t = instruction.t();
    let s = instruction.s();

    let v = self.reg(s).wrapping_add(i);
    self.set_reg(t, v);
  }
}

struct Instruction(u32);

impl Instruction {
  fn function(&self) -> u32 {
    let Instruction(op) = self;
    op >> 26
  }

  fn s(&self) -> u32 {
    let Instruction(op) = self;
    (op >> 21) & 0x1F
  }

  fn t(&self) -> u32 {
    let Instruction(op) = self;
    (op >> 16) & 0x1F
  }

  fn imm(&self) -> u32 {
    let Instruction(op) = self;
    op & 0xFFFF
  }

  fn imm_se(&self) -> u32 {
    let Instruction(op) = self;
    let v = (op & 0xFFFF) as i16;
    v as u32
  }

  fn d(&self) -> u32 {
    let Instruction(op) = self;
    (op >> 11) & 0x1F
  }

  fn subfunction(&self) -> u32 {
    let Instruction(op) = self;
    op & 0x3F
  }

  fn shift(&self) -> u32 {
    let Instruction(op) = self;
    (op >> 6) & 0x1F
  }
}
