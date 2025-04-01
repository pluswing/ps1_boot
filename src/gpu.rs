pub struct Gpu {
  page_base_x: u8,
  page_base_y: u8,
  semi_transparency: u8,
  texture_depth: TextureDepth,
  dithering: bool,
  draw_to_display: bool,
  force_set_mask_bit: bool,
  preserve_masked_pixels: bool,
  field: Field,
  texture_disable: bool,
  hres: HorizontalRes,
  vres: VerticalRes,
  vmode: VMode,
  display_depth: DisplayDepth,
  interlaced: bool,
  display_disabled: bool,
  interrupt: bool,
  dma_direction: DmaDirection,
}

impl Gpu {
  pub fn new() -> Self {
    Self {
      page_base_x: 0,
      page_base_y: 0,
      semi_transparency: 0,
      texture_depth: TextureDepth::T4Bit,
      dithering: false,
      draw_to_display: false,
      force_set_mask_bit: false,
      preserve_masked_pixels: false,
      field: Field::Top,
      texture_disable: false,
      hres: HorizontalRes::from_fields(0, 0),
      vres: VerticalRes::Y240Lines,
      vmode: VMode::Ntsc,
      display_depth: DisplayDepth::D15Bits,
      interlaced: false,
      display_disabled: true,
      interrupt: false,
      dma_direction: DmaDirection::Off,
    }
  }

  pub fn status(&self) -> u32 {
    let r = (self.page_base_x as u32) << 0 |
    (self.page_base_y as u32) << 4 |
    (self.semi_transparency as u32) << 5 |
    (self.texture_depth as u32) << 7 |
    (self.dithering as u32) << 9 |
    (self.draw_to_display as u32) << 10 |
    (self.force_set_mask_bit as u32) << 11 |
    (self.preserve_masked_pixels as u32) << 12 |
    (self.field as u32) << 13 |
    (self.texture_disable as u32) << 15 |
    self.hres.into_status() |
    (self.vres as u32) << 19 |
    (self.vmode as u32) << 20 |
    (self.display_depth as u32) << 21 |
    (self.interlaced as u32) << 22 |
    (self.display_disabled as u32) << 23 |
    (self.interrupt as u32) << 24 |
    1 << 26 |
    1 << 27 |
    1 << 28 |
    (self.dma_direction as u32) << 29 |
    0 << 31;

    let dma_request = match self.dma_direction {
      DmaDirection::Off => 0,
      DmaDirection::Fifo => 1,
      DmaDirection::CpuToGp0 => (r >> 28) & 1,
      DmaDirection::VramToCpu => (r >> 27) & 1,
    };
    r | dma_request << 25
  }
}

#[derive(Debug, Clone, Copy)]
enum TextureDepth {
  T4Bit = 0,
  T8Bit = 1,
  T15Bit = 2,
}

#[derive(Debug, Clone, Copy)]
enum Field {
  Top = 0,
  Bottom = 1,
}

#[derive(Debug, Clone, Copy)]
struct HorizontalRes(u8);

impl HorizontalRes {
  fn from_fields(hr1: u8, hr2: u8) -> Self {
    let hr = (hr2 & 1) | (hr1 & 3) << 1;
    Self(hr)
  }

  fn into_status(self) -> u32 {
    let HorizontalRes(hr) = self;
    (hr as u32) << 16
  }
}

#[derive(Debug, Clone, Copy)]
enum VerticalRes {
  Y240Lines = 0,
  Y480Lines = 1,
}

#[derive(Debug, Clone, Copy)]
enum VMode {
  Ntsc = 0,
  Pal = 1,
}

#[derive(Debug, Clone, Copy)]
enum DisplayDepth {
  D15Bits = 0,
  D24Bits = 1,
}

#[derive(Debug, Clone, Copy)]
enum DmaDirection {
  Off = 0,
  Fifo = 1,
  CpuToGp0 = 2,
  VramToCpu = 3,
}
