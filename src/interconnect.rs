use crate::bios::Bios;


pub struct Interconnect {
  bios: Bios
}

impl Interconnect {
  pub fn new(bios: Bios) -> Self {
    Self {
      bios,
    }
  }

  pub fn load32(&self, addr: u32) -> u32 {
    if addr % 4 != 0 {
      panic!("Unalignd load32 address: {:08X}", addr);
    }
    if let Some(offset) = map::BIOS.contains(addr) {
      return self.bios.load32(offset);
    }
    panic!("unhandled load32 at address {:08X}", addr);
  }

  pub fn store32(&mut self, addr: u32, val: u32) {
    if addr % 4 != 0 {
      panic!("Unalignd store32 address: {:08X}", addr);
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
}
