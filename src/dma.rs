pub struct Dma {
  control: u32,

  irq_en: bool,
  channel_irq_en: u8,
  channel_irq_flags: u8,
  force_irq: bool,
  irq_dummy: u8,
}

impl Dma {
  pub fn new() -> Self {
    Self {
      control: 0x0765_4321,
      irq_en: false,
      channel_irq_en: 0,
      channel_irq_flags: 0,
      force_irq: false,
      irq_dummy: 0,
    }
  }

  pub fn control(&self) -> u32 {
    self.control
  }

  pub fn set_control(&mut self, val: u32) {
    self.control = val
  }

  fn irq(&self) -> bool {
    let channel_irq = self.channel_irq_flags & self.channel_irq_en;
    self.force_irq || (self.irq_en && channel_irq != 0)
  }

  pub fn interrupt(&self) -> u32 {
    self.irq_dummy as u32 |
      (self.force_irq as u32) << 15 |
      (self.channel_irq_en as u32) << 16 |
      (self.irq_en as u32) << 23 |
      (self.channel_irq_flags as u32) << 24 |
      (self.irq() as u32) << 31
  }

  pub fn set_interrupt(&mut self, val: u32) {
    self.irq_dummy = (val & 0x3F) as u8;
    self.force_irq = (val >> 15) & 1 != 0;
    self.channel_irq_en = ((val >> 16) & 0x7F) as u8;
    self.irq_en = (val >> 23) & 1 != 0;
    let ack = ((val >> 24) & 0x3F) as u8;
    self.channel_irq_flags = self.channel_irq_flags & !ack;
  }
}
