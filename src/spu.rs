use std::cmp;

fn decode_adpcm_block(block: &[u8; 16], decoded: &mut [i16; 28], old_sample: &mut i16, older_sample: &mut i16) {

  let shift = block[0] & 0x0F;
  let shift = if shift > 12 { 9 } else { shift };

  let filter = cmp::min(4, (block[0] >> 4) & 0x07);

  for sample_idx in 0..28 {
    let sample_byte = block[2 + sample_idx / 2];
    let sample_nibble = (sample_byte >> (2 * (sample_idx % 2))) & 0x0F;

    let raw_sample: i32 = (((sample_nibble as i8) << 4) >> 4).into();

    let shifted_sample = raw_sample << (12 - shift);

    let old = i32::from(*old_sample);
    let older = i32::from(*older_sample);

    let filtered_sample = match filter {
      0 => shifted_sample,
      1 => shifted_sample + (60 * old + 32) / 64,
      2 => shifted_sample + (115 * old - 52 * older + 32) / 64,
      3 => shifted_sample + (98 * old - 55 * older + 32) / 64,
      4 => shifted_sample + (122 * old - 60 * older + 32) / 64,
      _ => unreachable!("filter was clamped to [0, 4]")
    };

    let clamped_sample = filtered_sample.clamp(-0x8000, 0x7FFF) as i16;
    decoded[sample_idx] = clamped_sample;

    *older_sample = *old_sample;
    *old_sample = clamped_sample;
  }
}

struct Voice {
  start_address: u32,
  repeat_address: u32,
  current_address: u32,
  pitch_counter: u16,
  decode_buffer: [i16; 28],
  envelope: AdsrEnvelope,

  sample_rete: u16,
  current_buffer_idx: u8,
  current_sample: i16,
}

impl Voice {
  fn clock(&mut self, sound_ram: &[u8]) {
    let pitch_counter_step = cmp::min(0x4000, self.sample_rete);
    self.pitch_counter = self.pitch_counter + pitch_counter_step;

    while self.pitch_counter >= 0x1000 {
      self.pitch_counter = self.pitch_counter - 0x1000;
      self.current_buffer_idx = self.current_buffer_idx + 1;

      if self.current_buffer_idx == 28 {
        self.current_buffer_idx = 0;
        self.decode_next_block(sound_ram);
      }
    }
    self.current_sample = self.decode_buffer[self.current_buffer_idx as usize];
  }

  fn key_on(&mut self, sound_ram: &[u8]) {
    self.envelope.key_on();

    self.current_address = self.start_address;
    self.pitch_counter = 0;
    self.decode_next_block(sound_ram);
  }

  fn decode_next_block(&mut self, sound_ram: &[u8]) {
    let block = &sound_ram[self.current_address as usize..(self.current_address + 16) as usize];
    decode_adpcm_block(
      block,
      &mut self.decode_buffer,
      &mut self.decode_buffer[self.decode_buffer.len() - 1],
      &mut self.decode_buffer[self.decode_buffer.len() - 2]);

      let loop_end = block[1] & (1 << 0) != 0;
      let loop_repeat = block[1] & (1 << 1) != 0;
      let loop_start = block[1] & (1 << 2) != 0;

      if loop_start {
        self.repeat_address = self.current_address;
      }

      if loop_end {
        self.current_address = self.repeat_address;

        if !loop_repeat {
          self.envelope.volume = 0;
          self.envelope.key_off();
        }
      } else {
        self.current_address += 16;
      }
  }
}

struct AdsrEnvelope {
  volume: u32,
}

impl AdsrEnvelope {
  fn new() -> Self {
    Self {
      volume: 0
    }
  }
  fn key_on(&self) {
  }
  fn key_off(&self) {
  }
}
