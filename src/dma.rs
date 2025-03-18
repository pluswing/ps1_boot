pub struct Dma {
  control: u32,
}

impl Dma {
  pub fn new() -> Self {
    Self {
      control: 0x0765_4321,
    }
  }

  pub fn control(&self) -> u32 {
    self.control
  }

  pub fn set_control(&mut self, val: u32) {
    self.control = val
  }
}
