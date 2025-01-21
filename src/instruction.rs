pub struct Instruction(u32);

impl Instruction {
  fn function(self) -> u32 {
    let Instruction(op) = self;
    op >> 26
  }

  fn t(self) -> u32 {
    let Instruction(op) = self;
    (op >> 16) & 0x1F
  }

  fn imm(self) -> u32 {
    let Instruction(op) = self;
    op & 0xFFFF
  }
}
