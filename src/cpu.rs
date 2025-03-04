use crate::interconnect::Interconnect;

pub struct Cpu {
  pc: u32,
  next_pc: u32,
  regs: [u32; 32],
  out_regs: [u32; 32],
  inter: Interconnect,
  next_instruction: Instruction,

  // COP0
  sr: u32,
  current_pc: u32,
  cause: u32,
  epc: u32,

  load: (RegisterIndex, u32),

  hi: u32,
  lo: u32,
}

impl Cpu {
  pub fn new(inter: Interconnect) -> Self {
    let mut regs = [0xDEAD_BEEF; 32];
    regs[0] = 0;
    let pc = 0xBFC0_0000;
    Self {
      pc,
      next_pc: pc.wrapping_add(4),
      regs,
      out_regs: regs,
      inter,
      next_instruction: Instruction(0), // NOP
      sr: 0,
      current_pc: 0,
      cause: 0,
      epc: 0,
      load: (RegisterIndex(0), 0),
      hi: 0xDEAD_BEEF,
      lo: 0xDEAD_BEEF,
    }
  }

  pub fn run_next_instruction(&mut self) {
    // FIXME PCの取扱がなんか変な気がする。
    let instruction = Instruction(self.load32(self.pc));
    self.current_pc = self.pc;
    self.pc = self.next_pc;
    self.next_pc = self.next_pc.wrapping_add(4);

    let (reg, val) = self.load;
    self.set_reg(reg, val);
    self.load = (RegisterIndex(0), 0);
    self.decode_and_execute(instruction);
    self.regs = self.out_regs;
  }

  fn load32(&self, addr: u32) -> u32 {
    self.inter.load32(addr)
  }

  fn store32(&mut self, addr: u32, val: u32) {
    self.inter.store32(addr, val)
  }

  fn store16(&mut self, addr: u32, val: u16) {
    self.inter.store16(addr, val)
  }

  fn store8(&mut self, addr: u32, val: u8) {
    self.inter.store8(addr, val)
  }

  fn load8(&self, addr: u32) -> u8 {
    self.inter.load8(addr)
  }

  fn decode_and_execute(&mut self, instruction: Instruction) {
    match instruction.function() {
      0b000000 => match instruction.subfunction() {
        0b000000 => self.op_sll(instruction),
        0b000010 => self.op_srl(instruction),
        0b000011 => self.op_sra(instruction),
        0b100101 => self.op_or(instruction),
        0b001000 => self.op_jr(instruction),
        0b001001 => self.op_jalr(instruction),
        0b010000 => self.op_mfhi(instruction),
        0b010010 => self.op_mflo(instruction),
        0x11 => self.op_mthi(instruction),
        0x13 => self.op_mtlo(instruction),
        0x0C => self.op_syscall(instruction),
        0b011010 => self.op_div(instruction),
        0b011011 => self.op_divu(instruction),
        0b100000 => self.op_add(instruction),
        0b100100 => self.op_and(instruction),
        0b100001 => self.op_addu(instruction),
        0b100011 => self.op_subu(instruction),
        0b101010 => self.op_slt(instruction),
        0b101011 => self.op_sltu(instruction),
        _ => panic!("Unhandled instrcuntion {:08X} (sub: 0b{:06b})", instruction.0, instruction.subfunction()),
      },
      0b000001 => self.op_bxx(instruction),
      0b000010 => self.op_j(instruction),
      0b000011 => self.op_jal(instruction),
      0b000100 => self.op_beq(instruction),
      0b000101 => self.op_bne(instruction),
      0b000110 => self.op_blez(instruction),
      0b000111 => self.op_bgtz(instruction),
      0b001000 => self.op_addi(instruction),
      0b001001 => self.op_addiu(instruction),
      0b001010 => self.op_slti(instruction),
      0b001011 => self.op_sltiu(instruction),
      0b001100 => self.op_andi(instruction),
      0b001101 => self.op_ori(instruction),
      0b001111 => self.op_lui(instruction),
      0b010000 => self.op_cop0(instruction),
      0b100000 => self.op_lb(instruction),
      0b100011 => self.op_lw(instruction),
      0b100100 => self.op_lbu(instruction),
      0b101000 => self.op_sb(instruction),
      0b101011 => self.op_sw(instruction),
      0b101001 => self.op_sh(instruction),
      _ => panic!("Unhandled instruction {:08X} (f: 0b{:06b})", instruction.0, instruction.function()),
    }
  }

  fn reg(&self, index: RegisterIndex) -> u32 {
    self.regs[index.0 as usize]
  }

  fn set_reg(&mut self, index: RegisterIndex, val: u32) {
    self.out_regs[index.0 as usize] = val;
    self.out_regs[0] = 0;
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

  fn op_j(&mut self, instruction: Instruction) {
    let i = instruction.imm_jump();
    self.pc = (self.pc & 0xF000_0000) | (i << 2);
  }

  fn op_or(&mut self, instruction: Instruction) {
    let d = instruction.d();
    let s = instruction.s();
    let t = instruction.t();

    let v = self.reg(s) | self.reg(t);
    self.set_reg(d, v);
  }

  fn op_cop0(&mut self, instruction: Instruction) {
    match instruction.cop_opcode() {
      0b00100 => self.op_mtc0(instruction),
      0b00000 => self.op_mfc0(instruction),
      0b10000 => self.op_rfe(instruction),
      _ => panic!("Unhandled cop0 instruction {:08X} (op: 0b{:06b})", instruction.0, instruction.cop_opcode()),
    }
  }

  fn op_mtc0(&mut self, instruction: Instruction) {
    let cpu_r = instruction.t();
    let cop_r = instruction.d().0;

    let v = self.reg(cpu_r);

    match cop_r {
      3 | 5 | 6 | 7 | 9 | 11 => {
        // breakpoint registers
        if v != 0 {
          panic!("Unhandled write to cop0r{}", cop_r)
        }
      }
      12 => self.sr = v,
      13 => {
        // CAUSE register
        if v != 0 {
          panic!("Unhandled write to CAUSE register.")
        }
      }
      n => panic!("Unhandled cop0 register: {:08X}", n),
    }
  }

  fn branch(&mut self, offset: u32) {
    let offset = offset << 2;
    let mut pc = self.pc;
    pc = pc.wrapping_add(offset);
    pc = pc.wrapping_sub(4);
    self.pc = pc;
  }

  fn op_bne(&mut self, instruction: Instruction) {
    let i = instruction.imm_se();
    let s = instruction.s();
    let t = instruction.t();

    if self.reg(s) != self.reg(t) {
      self.branch(i);
    }
  }

  fn op_addi(&mut self, instruction: Instruction) {
    let i: i32 = instruction.imm_se() as i32;
    let t = instruction.t();
    let s = instruction.s();

    let s = self.reg(s) as i32;
    let v = match s.checked_add(i) {
      Some(v) => v as u32,
      None => panic!("ADDI overflow"),
    };
    self.set_reg(t, v);
  }

  fn op_lw(&mut self, instruction: Instruction) {
    if self.sr & 0x1_0000 != 0 {
      println!("Ignoring load while cache is isolated");
      return;
    }
    let i = instruction.imm_se();
    let t = instruction.t();
    let s = instruction.s();

    let addr = self.reg(s).wrapping_add(i);
    let v = self.load32(addr);
    self.load = (t, v);
  }

  fn op_sltu(&mut self, instruction: Instruction) {
    let d = instruction.d();
    let s = instruction.s();
    let t = instruction.t();

    let v = self.reg(s) < self.reg(t);
    self.set_reg(d, v as u32);
  }

  fn op_addu(&mut self, instruction: Instruction) {
    let s = instruction.s();
    let t = instruction.t();
    let d = instruction.d();

    let v = self.reg(s).wrapping_add(self.reg(t));
    self.set_reg(d, v);
  }

  fn op_sh(&mut self, instruction: Instruction) {

    if self.sr & 0x1_0000 != 0 {
      println!("Ignoring store while cache is isolated");
      return;
    }

    let i = instruction.imm_se();
    let t = instruction.t();
    let s = instruction.s();

    let addr = self.reg(s).wrapping_add(i);
    let v = self.reg(t);
    self.store16(addr, v as u16);
  }

  fn op_jal(&mut self, instruction: Instruction) {
    let ra = self.pc;
    self.set_reg(RegisterIndex(31), ra);
    self.op_j(instruction);
  }

  fn op_andi(&mut self, instruction: Instruction) {
    let i = instruction.imm();
    let t = instruction.t();
    let s = instruction.s();
    let v = self.reg(s) & i;
    self.set_reg(t, v);
  }

  fn op_sb(&mut self, instruction: Instruction) {
    if self.sr & 0x1_0000 != 0 {
      println!("Ignoring store while cache is isolated");
      return;
    }

    let i = instruction.imm_se();
    let t = instruction.t();
    let s = instruction.s();

    let addr = self.reg(s).wrapping_add(i);
    let v = self.reg(t);
    self.store8(addr, v as u8);
  }

  fn op_jr(&mut self, instruction: Instruction) {
    let s = instruction.s();
    self.pc = self.reg(s);
  }

  fn op_lb(&mut self, instruction: Instruction) {
    let i = instruction.imm_se();
    let t = instruction.t();
    let s = instruction.s();

    let addr = self.reg(s).wrapping_add(i);
    let v = self.load8(addr) as i8;
    self.load = (t, v as u32);
  }

  fn op_beq(&mut self, instruction: Instruction) {
    let i = instruction.imm_se();
    let s = instruction.s();
    let t = instruction.t();

    if self.reg(s) == self.reg(t) {
      self.branch(i);
    }
  }

  fn op_mfc0(&mut self, instruction: Instruction) {
    let cpu_r = instruction.t();
    let cop_r = instruction.d().0;

    let v = match cop_r {
      12 => self.sr,
      13 => self.cause,
      14 => self.epc,
      _ => panic!("Unhandled read from cop0r{}", cop_r)
    };
    self.load = (cpu_r, v)
  }

  fn op_and(&mut self, instruction: Instruction) {
    let d = instruction.d();
    let s = instruction.s();
    let t = instruction.t();

    let v = self.reg(s) & self.reg(t);
    self.set_reg(d, v);
  }

  fn op_add(&mut self, instruction: Instruction) {
    let s = instruction.s();
    let t = instruction.t();
    let d = instruction.d();

    let s = self.reg(s) as i32;
    let t = self.reg(t) as i32;

    let v = match s.checked_add(t) {
      Some(v) => v as u32,
      None => panic!("ADD overflow"),
    };
    self.set_reg(d, v);
  }

  fn op_bgtz(&mut self, instruction: Instruction) {
    let i = instruction.imm_se();
    let s = instruction.s();

    let v = self.reg(s) as i32;
    if v > 0 {
      self.branch(i);
    }
  }

  fn op_blez(&mut self, instruction: Instruction) {
    let i = instruction.imm_se();
    let s = instruction.s();

    let v = self.reg(s) as i32;
    if v <= 0 {
      self.branch(i);
    }
  }


  fn op_lbu(&mut self, instruction: Instruction) {
    let i = instruction.imm_se();
    let t = instruction.t();
    let s = instruction.s();

    let addr = self.reg(s).wrapping_add(i);
    let v = self.load8(addr);
    self.load = (t, v as u32);
  }

  fn op_jalr(&mut self, instruction: Instruction) {
    let d = instruction.d();
    let s = instruction.s();
    let ra = self.pc;
    self.set_reg(d, ra);
    self.pc = self.reg(s);
  }

  // BGEZ, BLTZ, BGEZAL, BLTZAL => BcondZ
  fn op_bxx(&mut self, instruction: Instruction) {
    let i = instruction.imm_se();
    let s = instruction.s();

    let instruction = instruction.0;

    let is_bgez = (instruction >> 16) & 0x01;
    let is_link = (instruction>> 17) & 0x0F == 0x08;

    let v = self.reg(s) as i32;

    let test = (v < 0) as u32;

    let test = test ^ is_bgez;

    if is_link {
      let ra = self.pc;
      self.set_reg(RegisterIndex(31), ra);
    }

    if test != 0 {
      self.branch(i);
    }
  }

  fn op_slti(&mut self, instruction: Instruction) {
    let i = instruction.imm_se() as i32;
    let s = instruction.s();
    let t = instruction.t();

    let v = (self.reg(s) as i32) < i;
    self.set_reg(t, v as u32);
  }

  fn op_subu(&mut self, instruction: Instruction) {
    let s = instruction.s();
    let t = instruction.t();
    let d = instruction.d();

    let v = self.reg(s).wrapping_sub(self.reg(t));
    self.set_reg(d, v);
  }

  fn op_sra(&mut self, instruction: Instruction) {
    let i = instruction.shift();
    let t = instruction.t();
    let d = instruction.d();

    let v = (self.reg(t) as i32) >> i;
    self.set_reg(d, v as u32);
  }

  fn op_div(&mut self, instruction: Instruction) {
    let s = instruction.s();
    let t = instruction.t();

    let n = self.reg(s) as i32;
    let d = self.reg(t) as i32;

    if d == 0 {
      self.hi = n as u32;
      self.lo = if n >= 0 {
        0xFFFF_FFFF
      } else {
        0x0000_0001
      };
    } else if n as u32 == 0x8000_0000 && d == -1 {
      self.hi = 0;
      self.lo = 0x8000_0000;
    } else {
      self.hi = (n % d) as u32;
      self.lo = (n / d) as u32;
    }
  }

  fn op_mflo(&mut self, instruction: Instruction) {
    let d = instruction.d();
    let lo = self.lo;
    self.set_reg(d, lo);
  }

  fn op_srl(&mut self, instruction: Instruction) {
    let i = instruction.shift();
    let t = instruction.t();
    let d = instruction.d();

    let v = self.reg(t) >> i;
    self.set_reg(d, v);
  }

  fn op_sltiu(&mut self, instruction: Instruction) {
    let i = instruction.imm_se();
    let s = instruction.s();
    let t = instruction.t();

    let v = self.reg(s) < i;
    self.set_reg(t, v as u32);
  }


  fn op_divu(&mut self, instruction: Instruction) {
    let s = instruction.s();
    let t = instruction.t();

    let n = self.reg(s);
    let d = self.reg(t);

    if d == 0 {
      self.hi = n;
      self.lo = 0xFFFF_FFFF;
    } else {
      self.hi = n % d;
      self.lo = n / d;
    }
  }

  fn op_mfhi(&mut self, instruction: Instruction) {
    let d = instruction.d();
    let hi = self.hi;
    self.set_reg(d, hi);
  }

  fn op_slt(&mut self, instruction: Instruction) {
    let d = instruction.d();
    let s = instruction.s();
    let t = instruction.t();

    let s = self.reg(s) as i32;
    let t = self.reg(t) as i32;

    let v = s < t;
    self.set_reg(d, v as u32);
  }

  fn exception(&mut self, cause: Exception) {
    let handler = match self.sr & (1 << 22) != 0 {
      true => 0xBFC0_0180,
      false => 0x8000_0080,
    };

    let mode = self.sr & 0x3F;
    self.sr = self.sr & (!0x3F);
    self.sr = self.sr | ((mode << 2) & 0x3F);

    self.cause = (cause as u32) << 2;

    self.epc = self.current_pc;

    self.pc = handler;
    self.next_pc = self.pc.wrapping_add(4);
  }

  fn op_syscall(&mut self, _: Instruction) {
    self.exception(Exception::SysCall);
  }

  fn op_mtlo(&mut self, instruction: Instruction) {
    let s = instruction.s();
    self.lo = self.reg(s);
  }

  fn op_mthi(&mut self, instruction: Instruction) {
    let s = instruction.s();
    self.hi = self.reg(s);
  }

  fn op_rfe(&mut self, instruction: Instruction) {
    if instruction.0 & 0x3F != 0b010000 {
      panic!("Invalid cop0 instruction: {:?}", instruction);
    }

    let mode = self.sr & 0x3F;
    self.sr = self.sr & !0x3F;
    self.sr = self.sr | mode >> 2;
  }
}

#[derive(Debug, Clone, Copy)]
struct RegisterIndex(u32);

#[derive(Debug, Clone, Copy)]
struct Instruction(u32);

impl Instruction {
  fn function(&self) -> u32 {
    let Instruction(op) = self;
    op >> 26
  }

  fn s(&self) -> RegisterIndex {
    let Instruction(op) = self;
    RegisterIndex((op >> 21) & 0x1F)
  }

  fn t(&self) -> RegisterIndex {
    let Instruction(op) = self;
    RegisterIndex((op >> 16) & 0x1F)
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

  fn d(&self) -> RegisterIndex {
    let Instruction(op) = self;
    RegisterIndex((op >> 11) & 0x1F)
  }

  fn subfunction(&self) -> u32 {
    let Instruction(op) = self;
    op & 0x3F
  }

  fn shift(&self) -> u32 {
    let Instruction(op) = self;
    (op >> 6) & 0x1F
  }

  fn imm_jump(&self) -> u32 {
    let Instruction(op) = self;
    op & 0x03FF_FFFF
  }

  fn cop_opcode(&self) -> u32 {
    let Instruction(op) = self;
    (op >> 21) & 0x1F
  }

}

enum Exception {
  SysCall = 0x08,
}
