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

  rectangle_texture_x_flip: bool,
  rectangle_texture_y_flip: bool,

  texture_window_x_mask: u8,
  texture_window_y_mask: u8,
  texture_window_x_offset: u8,
  texture_window_y_offset: u8,
  drawing_area_left: u16,
  drawing_area_top: u16,
  drawing_area_right: u16,
  drawing_area_bottom: u16,
  drawing_x_offset: i16,
  drawing_y_offset: i16,
  display_vram_x_start: u16,
  display_vram_y_start: u16,
  display_horiz_start: u16,
  display_horiz_end: u16,
  display_line_start: u16,
  display_line_end: u16,
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

      rectangle_texture_x_flip: false,
      rectangle_texture_y_flip: false,

      texture_window_x_mask: 0,
      texture_window_y_mask: 0,
      texture_window_x_offset: 0,
      texture_window_y_offset: 0,
      drawing_area_left: 0,
      drawing_area_top: 0,
      drawing_area_right: 0,
      drawing_area_bottom: 0,
      drawing_x_offset: 0,
      drawing_y_offset: 0,
      display_vram_x_start: 0,
      display_vram_y_start: 0,
      display_horiz_start: 0,
      display_horiz_end: 0,
      display_line_start: 0,
      display_line_end: 0,
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

  pub fn gp0(&mut self, val: u32) {
    let opcode = (val >> 24) & 0xFF;
    match opcode {
      0x00 => (), // NOP
      0xE1 => self.gp0_draw_mode(val),
      _ => panic!("Unhandled GP0 command {:08X}", val)
    }
  }

  fn gp0_draw_mode(&mut self, val: u32) {
    self.page_base_x = (val & 0x0F) as u8;
    self.page_base_y = ((val >> 4) & 1) as u8;
    self.semi_transparency = ((val >> 5) & 3) as u8;

    self.texture_depth = match (val >> 7) & 3 {
      0 => TextureDepth::T4Bit,
      1 => TextureDepth::T8Bit,
      2 => TextureDepth::T15Bit,
      n => panic!("Unhandled texture depth {}", n),
    };
    self.dithering = ((val >> 9) & 1) != 0;
    self.draw_to_display = ((val >> 10) & 1) != 0;
    self.texture_disable = ((val >> 11) & 1) != 0;
    self.rectangle_texture_x_flip = ((val >> 12) & 1) != 0;
    self.rectangle_texture_y_flip = ((val >> 13) & 1) != 0;
  }

  pub fn gp1(&mut self, val: u32) {
    let opcode = (val >> 24) & 0xFF;
    match opcode {
      0x00 => self.gp1_reset(val),
      _ => panic!("Unhandled GP0 command {:08X}", val)
    }
  }

  fn gp1_reset(&mut self, _: u32) {
    self.interrupt = false;

    self.page_base_x = 0;
    self.page_base_y = 0;
    self.semi_transparency = 0;
    self.texture_depth = TextureDepth::T4Bit;
    self.texture_window_x_mask = 0;
    self.texture_window_y_mask = 0;
    self.texture_window_x_offset = 0;
    self.texture_window_y_offset = 0;
    self.dithering = false;
    self.draw_to_display = false;
    self.texture_disable = false;
    self.rectangle_texture_x_flip = false;
    self.rectangle_texture_y_flip = false;
    self.drawing_area_left = 0;
    self.drawing_area_top = 0;
    self.drawing_area_right = 0;
    self.drawing_area_bottom = 0;
    self.drawing_x_offset = 0;
    self.drawing_y_offset = 0;
    self.force_set_mask_bit = false;
    self.preserve_masked_pixels = false;
    self.dma_direction = DmaDirection::Off;
    self.display_disabled = true;
    self.display_vram_x_start = 0;
    self.display_vram_y_start = 0;
    self.hres = HorizontalRes::from_fields(0, 0);
    self.vres = VerticalRes::Y240Lines;

    self.vmode = VMode::Ntsc;
    self.interlaced = true;
    self.display_horiz_start = 0x0200;
    self.display_horiz_end = 0x0C00;
    self.display_line_start = 0x0010;
    self.display_line_end = 0x0100;
    self.display_depth = DisplayDepth::D15Bits;

    // XXX clear command FIFO
    // XXX invalidate GPU cache
  }

  pub fn read(&self) -> u32 {
    0
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
  Top = 1,
  Bottom = 0,
}

#[derive(Debug, Clone, Copy)]
struct HorizontalRes(u8);

impl HorizontalRes {
  fn from_fields(hr1: u8, hr2: u8) -> Self {
    let hr = (hr2 & 1) | ((hr1 & 3) << 1);
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
