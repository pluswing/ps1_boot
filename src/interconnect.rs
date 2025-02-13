use crate::{bios::Bios, ram::Ram};


pub struct Interconnect {
  bios: Bios,
  ram: Ram,
}

impl Interconnect {
  pub fn new(bios: Bios) -> Self {
    Self {
      bios,
      ram: Ram::new(),
    }
  }

  pub fn load32(&self, addr: u32) -> u32 {
    if addr % 4 != 0 {
      panic!("Unalignd load32 address: {:08X}", addr);
    }
    if let Some(offset) = map::BIOS.contains(addr) {
      return self.bios.load32(offset);
    }
    if let Some(offset) = map::RAM.contains(addr) {
      return self.ram.load32(offset);
    }
    panic!("unhandled load32 at address {:08X}", addr);
  }

  pub fn store32(&mut self, addr: u32, val: u32) {
    if addr % 4 != 0 {
      panic!("Unalignd store32 address: {:08X}", addr);
    }

    if let Some(offset) = map::MEM_CONTROL.contains(addr) {
      match offset {
        0 => if val != 0x1F00_0000 {
          panic!("Bad expansion 1 base address: 0x{:01X}", val);
        }
        4 => if val != 0x1F80_2000 {
          panic!("Bad expansion 2 base address: 0x{:01X}", val);
        }
        _ => println!("Unhandled write to MEM_CONTROL register"),
      }
      return;
    }
    if let Some(_) = map::RAM_SIZE.contains(addr) {
      println!("write to RAM_SIZE register {:08X}", addr);
      return;
    }
    if let Some(_) = map::CACHE_CONTROL.contains(addr) {
      println!("write to CACHE_CONTROL register {:08X}", addr);
      return;
    }
    if let Some(offset) = map::RAM.contains(addr) {
      self.ram.store32(offset, val);
      return;
    }
    panic!("unhandled store32 at address {:08X}", addr)
  }
}

mod map {
  pub struct Range(u32, u32);

  impl Range {

    pub fn contains(self, addr: u32) -> Option<u32> {
      let Range(start, length) = self;
      if addr >= start && addr < start + length {
        Some(addr - start)
      } else {
        None
      }
    }
  }

  pub const BIOS: Range = Range(0xBFC0_0000, 512 * 1024);
  pub const MEM_CONTROL: Range = Range(0x1F80_1000, 36);
  pub const RAM_SIZE: Range = Range(0x1F80_1060, 4);
  pub const CACHE_CONTROL: Range = Range(0xFFFE_0130, 4);
  pub const RAM: Range = Range(0xA000_0000, 2 * 1024 * 1024);
}
