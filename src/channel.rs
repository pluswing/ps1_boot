#[derive(Debug, Clone, Copy)]
pub struct Channel {
  enable: bool,
  direction: Direction,
  step: Step,
  sync: Sync,
  trigger: bool,
  chop: bool,
  chop_dma_sz: u8,
  chop_cpu_sz: u8,
  dummy: u8,
}

impl Channel {
  pub fn new() -> Self {
    Self {
      enable: false,
      direction: Direction::ToRam,
      step: Step::Increment,
      sync: Sync::Manual,
      trigger: false,
      chop: false,
      chop_dma_sz: 0,
      chop_cpu_sz: 0,
      dummy: 0,
    }
  }

  pub fn control(&self) -> u32 {
    (self.direction as u32) << 0 |
    (self.step as u32) << 1 |
    (self.chop as u32) << 8 |
    (self.sync as u32) << 9 |
    (self.chop_dma_sz as u32) << 16 |
    (self.chop_cpu_sz as u32) << 20 |
    (self.enable as u32) << 24 |
    (self.trigger as u32) << 28 |
    (self.dummy as u32) << 29
  }

  pub fn set_control(&mut self, val: u32) {
    self.direction = match (val & 1) != 0 {
      true => Direction::FromRam,
      false => Direction::ToRam,
    };
    self.step = match (val >> 1) & 1 != 0 {
      true => Step::Decrement,
      false => Step::Increment,
    };
    self.chop = (val >> 8) & 1 != 0;
    self.sync = match (val >> 9) & 3 {
      0 => Sync::Manual,
      1 => Sync::Request,
      2 => Sync::LinkedList,
      n => panic!("Unknown DMA sync mode: {}", n),
    };
    self.chop_dma_sz = ((val >> 16) & 7) as u8;
    self.chop_cpu_sz = ((val >> 20) & 7) as u8;

    self.enable = (val >> 24) & 1 != 0;
    self.trigger = (val >> 28) & 1 != 0;
    self.dummy = ((val >> 29) & 3) as u8;
  }
}

#[derive(Debug, Clone, Copy)]
pub enum Direction {
  ToRam = 0,
  FromRam = 1,
}

#[derive(Debug, Clone, Copy)]
pub enum Step {
  Increment = 0,
  Decrement = 1,
}

#[derive(Debug, Clone, Copy)]
pub enum Sync {
  Manual = 0,
  Request = 1,
  LinkedList = 2,
}

