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

  branch: bool,
  delay_slot: bool,
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
      branch: false,
      delay_slot: false,
    }
  }

  pub fn run_next_instruction(&mut self) {
    self.current_pc = self.pc;
    if self.current_pc % 4 != 0 {
      self.exception(Exception::LoadAddressError);
      return;
    }
    let instruction = Instruction(self.load32(self.pc));
    // println!("PC: {:08X} => {:02X} ({:02X})", self.pc, instruction.function(), instruction.subfunction());
    self.pc = self.next_pc;
    self.next_pc = self.next_pc.wrapping_add(4);

    let (reg, val) = self.load;
    self.set_reg(reg, val);
    self.load = (RegisterIndex(0), 0);
    self.delay_slot = self.branch;
    self.branch = false;
    self.decode_and_execute(instruction);
    self.regs = self.out_regs;
  }

  fn load32(&self, addr: u32) -> u32 {
    self.inter.load32(addr)
  }

  fn store32(&mut self, addr: u32, val: u32) {
    self.inter.store32(addr, val)
  }

  fn load16(&self, addr: u32) -> u16 {
    self.inter.load16(addr)
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
        0x00 => self.op_sll(instruction),
        0x02 => self.op_srl(instruction),
        0x03 => self.op_sra(instruction),
        0x04 => self.op_sllv(instruction),
        0x06 => self.op_srlv(instruction),
        0x07 => self.op_srav(instruction),
        0x08 => self.op_jr(instruction),
        0x09 => self.op_jalr(instruction),
        0x0C => self.op_syscall(instruction),
        0x0D => self.op_break(instruction),
        0x10 => self.op_mfhi(instruction),
        0x11 => self.op_mthi(instruction),
        0x12 => self.op_mflo(instruction),
        0x13 => self.op_mtlo(instruction),
        0x18 => self.op_mult(instruction),
        0x19 => self.op_multu(instruction),
        0x1A => self.op_div(instruction),
        0x1B => self.op_divu(instruction),
        0x20 => self.op_add(instruction),
        0x21 => self.op_addu(instruction),
        0x22 => self.op_sub(instruction),
        0x23 => self.op_subu(instruction),
        0x24 => self.op_and(instruction),
        0x25 => self.op_or(instruction),
        0x26 => self.op_xor(instruction),
        0x27 => self.op_nor(instruction),
        0x2A => self.op_slt(instruction),
        0x2B => self.op_sltu(instruction),
        _ => panic!("Unhandled instrcuntion {:08X} (sub: 0b{:06b})", instruction.0, instruction.subfunction()),
      },
      0x01 => self.op_bxx(instruction),
      0x02 => self.op_j(instruction),
      0x03 => self.op_jal(instruction),
      0x04 => self.op_beq(instruction),
      0x05 => self.op_bne(instruction),
      0x06 => self.op_blez(instruction),
      0x07 => self.op_bgtz(instruction),
      0x08 => self.op_addi(instruction),
      0x09 => self.op_addiu(instruction),
      0x0A => self.op_slti(instruction),
      0x0B => self.op_sltiu(instruction),
      0x0C => self.op_andi(instruction),
      0x0D => self.op_ori(instruction),
      0x0E => self.op_xori(instruction),
      0x0F => self.op_lui(instruction),
      0x10 => self.op_cop0(instruction),
      0x11 => self.op_cop1(instruction),
      0x12 => self.op_cop2(instruction),
      0x13 => self.op_cop3(instruction),
      0x20 => self.op_lb(instruction),
      0x21 => self.op_lh(instruction),
      0x22 => self.op_lwl(instruction),
      0x23 => self.op_lw(instruction),
      0x24 => self.op_lbu(instruction),
      0x25 => self.op_lhu(instruction),
      0x26 => self.op_lwr(instruction),
      0x28 => self.op_sb(instruction),
      0x29 => self.op_sh(instruction),
      0x2A => self.op_swl(instruction),
      0x2B => self.op_sw(instruction),
      0x2E => self.op_swr(instruction),
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
    if addr % 4 == 0 {
      let v = self.reg(t);
      self.store32(addr, v);
    } else {
      self.exception(Exception::StoreAddressError);
    }
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
    self.next_pc = (self.pc & 0xF000_0000) | (i << 2);
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
    // FIXME next_pcを導入したことによりおそらく不要になっている
    // pc = pc.wrapping_sub(4);
    self.next_pc = pc;
    self.branch = true;
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
    match s.checked_add(i) {
      Some(v) => self.set_reg(t, v as u32),
      None => self.exception(Exception::Overflow),
    }
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
    if addr % 4 == 0 {
      let v = self.load32(addr);
      self.load = (t, v);
    } else {
      self.exception(Exception::LoadAddressError);
    }
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
    if addr % 2 == 0 {
      let v = self.reg(t);
      self.store16(addr, v as u16);
    } else {
      self.exception(Exception::StoreAddressError);
    }
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
    self.next_pc = self.reg(s);
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

    match s.checked_add(t) {
      Some(v) => self.set_reg(d, v as u32),
      None => self.exception(Exception::Overflow),
    }
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
    self.next_pc = self.reg(s);
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

    if self.delay_slot {
      self.epc = self.epc.wrapping_sub(4);
      self.cause = self.cause | (1 << 31);
    }

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

  fn op_lhu(&mut self, instruction: Instruction) {
    let i = instruction.imm_se();
    let t = instruction.t();
    let s = instruction.s();

    let addr = self.reg(s).wrapping_add(i);
    if addr % 2 == 0 {
      let v = self.load16(addr);
      self.load = (t, v as u32);
    } else {
      self.exception(Exception::LoadAddressError);
    }
  }

  fn op_sllv(&mut self, instruction: Instruction) {
    let d = instruction.d();
    let s = instruction.s();
    let t = instruction.t();

    let v = self.reg(t) << (self.reg(s) & 0x1F);
    self.set_reg(d, v);
  }

  fn op_lh(&mut self, instruction: Instruction) {
    let i = instruction.imm_se();
    let t = instruction.t();
    let s = instruction.s();

    let addr = self.reg(s).wrapping_add(i);
    let v = self.load16(addr) as i16;
    self.load = (t, v as u32);
  }

  fn op_nor(&mut self, instruction: Instruction) {
    let d = instruction.d();
    let s = instruction.s();
    let t = instruction.t();
    let v = !(self.reg(s) | self.reg(t));
    self.set_reg(d, v);
  }

  fn op_srav(&mut self, instruction: Instruction) {
    let d = instruction.d();
    let s = instruction.s();
    let t = instruction.t();

    let v = (self.reg(t) as i32) >> (self.reg(s) & 0x1F);
    self.set_reg(d, v as u32);
  }

  fn op_srlv(&mut self, instruction: Instruction) {
    let d = instruction.d();
    let s = instruction.s();
    let t = instruction.t();

    let v = self.reg(t) >> (self.reg(s) & 0x1F);
    self.set_reg(d, v);
  }

  fn op_multu(&mut self, instruction: Instruction) {
    let s = instruction.s();
    let t = instruction.t();

    let a = self.reg(s) as u64;
    let b = self.reg(t) as u64;
    let v = a * b;

    self.hi = (v >> 32) as u32;
    self.lo = v as u32;
  }

  fn op_xor(&mut self, instruction: Instruction) {
    let d = instruction.d();
    let s = instruction.s();
    let t = instruction.t();

    let v = self.reg(s) ^ self.reg(t);
    self.set_reg(d, v);
  }

  fn op_break(&mut self, instruction: Instruction) {
    self.exception(Exception::Break);
  }

  fn op_mult(&mut self, instruction: Instruction) {
    let s = instruction.s();
    let t = instruction.t();

    let a = (self.reg(s) as i32) as i64;
    let b = (self.reg(t) as i32) as i64;
    let v = (a * b) as u64;

    self.hi = (v >> 32) as u32;
    self.lo = v as u32;
  }

  fn op_sub(&mut self, instruction: Instruction) {
    let s = instruction.s();
    let t = instruction.t();
    let d = instruction.d();

    let s = self.reg(s) as i32;
    let t = self.reg(t) as i32;

    match s.checked_sub(t) {
      Some(v) => self.set_reg(d, v as u32),
      None => self.exception(Exception::Overflow),
    }
  }

  fn op_xori(&mut self, instruction: Instruction) {
    let i = instruction.imm();
    let t = instruction.t();
    let s = instruction.s();
    let v = self.reg(s) ^ i;
    self.set_reg(t, v);
  }

  fn op_cop1(&mut self, _: Instruction) {
    self.exception(Exception::CoprocessorError);
  }

  fn op_cop3(&mut self, _: Instruction) {
    self.exception(Exception::CoprocessorError);
  }

  fn op_cop2(&mut self, instruction: Instruction) {
    panic!("unhandled GTE instruction: {:?}", instruction)
  }

  fn op_lwl(&mut self, instruction: Instruction) {
    let i = instruction.imm_se();
    let t = instruction.t();
    let s = instruction.s();

    let addr = self.reg(s).wrapping_add(i);
    let cur_v = self.out_regs[t.0 as usize];

    let aligned_addr = addr & !0x03;
    let aligned_word = self.load32(aligned_addr);

    let v = match addr & 0x03 {
      0 => (cur_v & 0x00FFFFFF) | (aligned_word << 24),
      1 => (cur_v & 0x0000FFFF) | (aligned_word << 16),
      2 => (cur_v & 0x000000FF) | (aligned_word << 8),
      3 => (cur_v & 0x00000000) | (aligned_word << 0),
      _ => unreachable!(),
    };

    self.load = (t, v);
  }

  fn op_lwr(&mut self, instruction: Instruction) {
    let i = instruction.imm_se();
    let t = instruction.t();
    let s = instruction.s();

    let addr = self.reg(s).wrapping_add(i);
    let cur_v = self.out_regs[t.0 as usize];

    let aligned_addr = addr & !0x03;
    let aligned_word = self.load32(aligned_addr);

    let v = match addr & 0x03 {
      0 => (cur_v & 0x00000000) | (aligned_word >> 0),
      1 => (cur_v & 0xFF000000) | (aligned_word >> 8),
      2 => (cur_v & 0xFFFF0000) | (aligned_word >> 16),
      3 => (cur_v & 0xFFFFFF00) | (aligned_word >> 24),
      _ => unreachable!(),
    };

    self.load = (t, v);
  }

  fn op_swl(&mut self, instruction: Instruction) {
    let i = instruction.imm_se();
    let t = instruction.t();
    let s = instruction.s();

    let addr = self.reg(s).wrapping_add(i);
    let v = self.reg(t);

    let aligned_addr = addr & !0x03;
    let cur_mem = self.load32(aligned_addr);

    let mem = match addr & 0x03 {
      0 => (cur_mem & 0xFF000000) | (v >> 24),
      1 => (cur_mem & 0xFFFF0000) | (v >> 16),
      2 => (cur_mem & 0xFF000000) | (v >> 8),
      3 => (cur_mem & 0x00000000) | (v >> 0),
      _ => unreachable!(),
    };

    self.store32(aligned_addr, mem);
  }

  fn op_swr(&mut self, instruction: Instruction) {
    let i = instruction.imm_se();
    let t = instruction.t();
    let s = instruction.s();

    let addr = self.reg(s).wrapping_add(i);
    let v = self.reg(t);

    let aligned_addr = addr & !0x03;
    let cur_mem = self.load32(aligned_addr);

    let mem = match addr & 0x03 {
      0 => (cur_mem & 0x00000000) | (v << 0),
      1 => (cur_mem & 0x000000FF) | (v << 8),
      2 => (cur_mem & 0x0000FFFF) | (v << 16),
      3 => (cur_mem & 0x00FFFFFF) | (v << 24),
      _ => unreachable!(),
    };

    self.store32(aligned_addr, mem);
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
  LoadAddressError = 0x04,
  StoreAddressError = 0x05,
  SysCall = 0x08,
  Break = 0x09,
  CoprocessorError = 0x0B,
  Overflow = 0x0C,
}
