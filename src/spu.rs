use std::cmp;

use sdl2::audio::{AudioQueue, AudioSpecDesired};

fn decode_adpcm_block(block: &[u8], decoded: &mut [i16; 28], old_sample: &mut i16, older_sample: &mut i16) {

  let shift = block[0] & 0x0F;
  let shift = if shift > 12 { 9 } else { shift };

  let filter = cmp::min(4, (block[0] >> 4) & 0x07);

  for sample_idx in 0..28 {
    let sample_byte = block[2 + sample_idx / 2];
    let sample_nibble = (sample_byte >> (4 * (sample_idx % 2))) & 0x0F;

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

pub struct Spu {
  voices: [Voice; 24],
  device: AudioQueue<i16>,

  sound_ram: [u8; 512 * 1024],
  sound_ram_start_address: u32,
  main_volume_l: i16,
  main_volume_r: i16,
  write_count: u32, // for debug
}

impl Spu {
  pub fn new(audio_subsystem: sdl2::AudioSubsystem) -> Self {
    let desired_spec = AudioSpecDesired {
      freq: Some(44100),
      channels: Some(2),
      samples: None,
    };
    let device: AudioQueue<i16> = audio_subsystem
        .open_queue::<i16, _>(None, &desired_spec)
        .unwrap();
    device.resume();
    Self {
      voices: [Voice::new(); 24],
      device,
      sound_ram: [0; 512 * 1024],
      sound_ram_start_address: 0x00,
      write_count: 0,
      main_volume_l: 0x7FFF,
      main_volume_r: 0x7FFF,
    }
  }

  pub fn load(&self, abs_addr: u32, offset: u32) -> u16 {
    match offset {
      0x01AC => { // 0x1F801DAC サウンド RAM データ転送制御
        0x0004
      }
      _ => 0
    }
  }

  pub fn store(&mut self, abs_addr: u32, offset: u32, val: u16) {
    match offset {
      0x0000..=0x017F => {  // 0x1F801C00..=0x1F801D7F
        let index = (offset / 0x10) as usize;
        self.voices[index].store(offset % 0x10, val);
      }
      0x01A6 => { // 0x1F801DA6
        // サウンドRAMデータポート開始アドレス
        self.sound_ram_start_address = (val as u32) << 3;
        self.write_count = 0;
      }
      0x01A8 => { // 1F801DA8
        // サウンド RAM データ ポート (16 ビット)
        self.sound_ram[self.sound_ram_start_address as usize] = val as u8;
        self.sound_ram[(self.sound_ram_start_address + 1) as usize] = (val >> 8) as u8;
        self.sound_ram_start_address = self.sound_ram_start_address + 2;
        self.write_count = self.write_count + 1;
      }
      0x0188 => { // 0x1F801D88
        // キーオンボイス0～15（0=変更なし、1=キーオン）
        for (i, voice) in self.voices[0..15].iter_mut().enumerate() {
          let flag = (val & 0x01 << i) != 0;
          if flag {
            println!("VOICE KEY ON[{}]", i);
            voice.key_on(&self.sound_ram);
          }
        }
      }
      0x018A => { // 0x1F801D8A
        // キーオンボイス16～23のキー
        for (i, voice) in self.voices[15..].iter_mut().enumerate() {
          let flag = (val & 0x01 << i) != 0;
          if flag {
            println!("VOICE KEY ON[{}]", i+15);
            voice.key_on(&self.sound_ram);
          }
        }
      }
      0x018C => { // 0x1F801D8C
        // キーオフボイス 0-15 (0=変更なし、1=キーオフ)
        for (i, voice) in self.voices[0..15].iter_mut().enumerate() {
          let flag = (val & 0x01 << i) != 0;
          if flag {
            println!("VOICE KEY OFF[{}]", i);
            voice.key_off();
          }
        }
      }
      0x018E => { // 0x1F801D8E
        // キーオフボイス16-23
        for (i, voice) in self.voices[15..].iter_mut().enumerate() {
          let flag = (val & 0x01 << i) != 0;
          if flag {
            println!("VOICE KEY OFF[{}]", i+15);
            voice.key_off();
          }
        }
      }
      0x01B8 => { // 0x1F801DB8 メイン左ボリューム
        if val & 0x8000 != 0 {
          // エンベロープ有効
          println!("MAIN LEFT VOLUME *ENVELOPE* {:04X}", val)
        } else {
          // 一定音量
          self.main_volume_l = (val << 1) as i16;
        }
      }
      0x01BA => { // 0x1F801DB8 メイン右ボリューム
        if val & 0x8000 != 0 {
          // エンベロープ有効
          println!("MAIN RIGHT VOLUME *ENVELOPE* {:04X}", val)
        } else {
          // 一定音量
          self.main_volume_r = (val << 1) as i16;
        }
      }
      _ => {
        // println!("Unhandled SPU store: {:08X}({:04X}) {:04X}", abs_addr, offset, val);
      }
    }
  }

  pub fn clock(&mut self) {

    let mut mixed_l = 0;
    let mut mixed_r = 0;
    for voice in &mut self.voices {
      if !voice.keyed_on {
        continue;
      }

      voice.clock(&self.sound_ram);

      let s = voice.current_sample;
      let (voice_sample_l, voice_sample_r) = voice.apply_voice_volume(s);
      mixed_l += i32::from(voice_sample_l);
      mixed_r += i32::from(voice_sample_r);
    }

    let clamped_l = mixed_l.clamp(-0x8000, 0x7FFF) as i16;
    let clamped_r = mixed_r.clamp(-0x8000, 0x7FFF) as i16;

    let output_l = apply_volume(clamped_l, self.main_volume_l);
    let output_r = apply_volume(clamped_r, self.main_volume_r);
    self.device.queue_audio(&[output_l, output_r]).unwrap()
  }
}

#[derive(Copy, Clone, Debug)]
struct Voice {
  start_address: u32,
  repeat_address: u32,
  current_address: u32,
  pitch_counter: u16,
  decode_buffer: [i16; 28],
  envelope: AdsrEnvelope,

  sample_rate: u16,
  current_buffer_idx: u8,
  current_sample: i16,

  keyed_on: bool,
  volume_l: i16,
  volume_r: i16,

  // envelope関連
  enable_envelope: bool,
  adsr1: u16,
  adsr2: u16,
}

impl Voice {
  fn new () -> Self {
    Self {
      start_address: 0,
      repeat_address: 0,
      current_address: 0,
      pitch_counter: 0,
      decode_buffer: [0; 28],
      envelope: AdsrEnvelope::new(),
      sample_rate: 0,
      current_buffer_idx: 0,
      current_sample: 0,
      keyed_on: true,
      volume_l: 0x7FFF,
      volume_r: 0x7FFF,

      enable_envelope: false,
      adsr1: 0,
      adsr2: 0,
    }
  }

  fn clock(&mut self, sound_ram: &[u8]) {

    if self.enable_envelope {
      let mut direction = Direction::Increasing;
      let mut rate = ChangeRate::Linear;
      let mut shift: u8 = 0;
      let mut step: u8 = 0;
      match self.envelope.phase {
        AdsrPhase::Attack => {
          // 15    Attack Mode       (0=Linear, 1=Exponential)
          // -     Attack Direction  (Fixed, always Increase) (until Level 7FFFh)
          // 14-10 Attack Shift      (0..1Fh = Fast..Slow)
          // 9-8   Attack Step       (0..3 = "+7,+6,+5,+4")
          rate = if self.adsr1 & 0x8000 == 0 { ChangeRate::Linear } else { ChangeRate::Exponential };
          direction = Direction::Increasing;
          shift = ((self.adsr1 & 0x7C00) >> 10) as u8;
          step = ((self.adsr1 & 0x0300) >> 8) as u8;
        }
        AdsrPhase::Decay => {
          // -     Decay Mode        (Fixed, always Exponential)
          // -     Decay Direction   (Fixed, always Decrease) (until Sustain Level)
          // 7-4   Decay Shift       (0..0Fh = Fast..Slow)
          // -     Decay Step        (Fixed, always "-8")
          rate = ChangeRate::Exponential;
          direction = Direction::Decreasing;
          shift = ((self.adsr1 & 0x0078) >> 3) as u8;
          step = 0;
        }
        AdsrPhase::Sustain => {
          // 3-0   Sustain Level     (0..0Fh)  ;Level=(N+1)*800h
          self.envelope.sustain_level = self.adsr1 & 0x0007;

          // 31    Sustain Mode      (0=Linear, 1=Exponential)
          // 30    Sustain Direction (0=Increase, 1=Decrease) (until Key OFF flag)
          // 29    Not used?         (should be zero)
          // 28-24 Sustain Shift     (0..1Fh = Fast..Slow)
          // 23-22 Sustain Step      (0..3 = "+7,+6,+5,+4" or "-8,-7,-6,-5") (inc/dec)
          rate = if self.adsr2 & 0x2000 == 0 { ChangeRate::Linear } else { ChangeRate::Exponential };
          direction = if self.adsr2 & 0x1000 == 0 { Direction::Increasing } else { Direction::Decreasing };
          shift = ((self.adsr2 & 0x0780) >> 9) as u8;
          step = ((self.adsr2 & 0x0060) >> 7) as u8;
        }
        AdsrPhase::Release => {
          // 21    Release Mode      (0=Linear, 1=Exponential)
          // -     Release Direction (Fixed, always Decrease) (until Level 0000h)
          // 20-16 Release Shift     (0..1Fh = Fast..Slow)
          // -     Release Step      (Fixed, always "-8")
          rate = if (self.adsr2 & 0x0010) != 0 { ChangeRate::Linear } else { ChangeRate::Exponential };
          direction = Direction::Decreasing;
          shift = (self.adsr2 & 0x000F) as u8;
          step = 0;
        }
      }
      self.envelope.check_for_phase_transition();
      self.envelope.update(direction, rate, shift, step);
      self.envelope.clock(direction, rate, shift);
    }

    let pitch_counter_step = cmp::min(0x4000, self.sample_rate);
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
    self.keyed_on = true;
  }

  fn key_off(&mut self) {
    self.envelope.key_off();
    self.keyed_on = false;
  }

  fn decode_next_block(&mut self, sound_ram: &[u8]) {
    let block = &sound_ram[self.current_address as usize..(self.current_address + 16) as usize];
    let mut old_sample = self.decode_buffer[self.decode_buffer.len() - 1];
    let mut older_sample = self.decode_buffer[self.decode_buffer.len() - 2];
    decode_adpcm_block(
      block,
      &mut self.decode_buffer,
      &mut old_sample,
      &mut older_sample);

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

  fn store(&mut self, offset: u32, val: u16) {
    match offset { /* 0x00 ~ 0x0F */
      0x00 => {
        // 左音量
        self.enable_envelope = val & 0x8000 != 0;
        self.volume_l = (val << 1) as i16;
      }
      0x02 => {
        // 右音量
        self.enable_envelope = val & 0x8000 != 0;
        self.volume_r = (val << 1) as i16;
      }
      0x04 => {
        // ADPCMサンプルレート
        self.sample_rate = val;
      }
      0x06 => {
        // ADPCM開始アドレス
        self.start_address = (val as u32) << 3;
      }
      0x08 => {
        // アタック設定、ディケイ設定、サステインレベル
        self.adsr1 = val;
      }
      0x0A => {
        // サステイン設定、リリース設定
        self.adsr2 = val;
      }
      0x0C => {
        // なにかしらある
      }
      0x0E => {
        // ADPCM繰り返しアドレス
        self.repeat_address = (val as u32) << 3;
      }
      _ => {
        // println!("Unhandled Voice Register {:04X} => {:04X}", offset, val);
      }
    }
  }

  fn apply_voice_volume(&self, adpcm_sample: i16) -> (i16, i16) {
    let envelope_sample = apply_volume(adpcm_sample, self.envelope.level);

    // TODO エンベロープを効かせる
    let output_l = apply_volume(envelope_sample, self.volume_l);
    let output_r = apply_volume(envelope_sample, self.volume_r);
    (output_l, output_r)
  }
}

fn apply_volume(sample: i16, volume: i16) -> i16 {
  ((i32::from(sample) * i32::from(volume)) >> 15) as i16
}

#[derive(Copy, Clone, Debug)]
struct AdsrEnvelope {
  volume: u32,
  level: i16,
  counter: u32,
  phase: AdsrPhase,
  sustain_level: u16,
}

impl AdsrEnvelope {
  fn new() -> Self {
    Self {
      volume: 0,
      level: 0x7FFF, // TODO MAX VOLUME
      counter: 0,
      phase: AdsrPhase::Release,
      sustain_level: 0,
    }
  }
  fn clock(&mut self, direction: Direction, rate: ChangeRate, shift: u8) {
    let mut counter_decrement = ENVELOPE_CONTER_MAX >> shift.saturating_sub(11);

    if direction == Direction::Increasing && rate == ChangeRate::Exponential && self.level > 0x6000 {
      counter_decrement >>= 2;
    }

    self.counter = self.counter.saturating_sub(counter_decrement);
    if self.counter == 0 {
      self.counter = ENVELOPE_CONTER_MAX;
      todo!("update envelope")
    }
  }

  fn update(&mut self, direction: Direction, rate: ChangeRate, shift: u8, step: u8) {
    let mut step = i32::from(7 - step);
    if direction == Direction::Decreasing {
      step = !step;
    }
    step <<= 11_u8.saturating_sub(shift);

    let current_level: i32 = self.level.into();
    if direction == Direction::Decreasing && rate == ChangeRate::Exponential {
      step = (step * current_level) >> 15;
    }
    self.level = (current_level + step).clamp(0, 0x7FFF) as i16;
  }

  fn key_on(&mut self) {
    self.level = 0;
    self.phase = AdsrPhase::Attack;
  }

  fn key_off(&mut self) {
    self.phase = AdsrPhase::Release;
  }

  fn check_for_phase_transition(&mut self) {
    if self.phase == AdsrPhase::Attack && self.level == 0x7FFF {
      self.phase = AdsrPhase::Decay;
    }

    if self.phase == AdsrPhase::Decay && (self.level as u16) <= self.sustain_level {
      self.phase = AdsrPhase::Sustain;
    }
  }

}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum Direction {
  Increasing,
  Decreasing,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum ChangeRate {
  Linear,
  Exponential,
}

const ENVELOPE_CONTER_MAX: u32 = 1 << (33 - 11);

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum AdsrPhase {
  Attack,
  Decay,
  Sustain,
  Release,
}
