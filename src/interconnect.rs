use crate::{bios::Bios, map};


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
    if let Some(offset) = map::BIOS.contains(addr) {
      return self.bios.load32(offset);
    }
    panic!("unhandled load32 at address {:08X}", addr);
  }
}
