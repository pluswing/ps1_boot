use std::{cmp, collections::VecDeque};

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

  reverb_start_address: u32,
  reverb_write_address: u32,
  reverb_output_volume_l: i16,
  reverb_output_volume_r: i16,
  reverb_input_volume_l: i16,
  reverb_input_volume_r: i16,
  reverb_left: bool,

  mlsame: u32,
  dlsame: u32,
  mrsame: u32,
  drsame: u32,
  mrdiff: u32,
  dldiff: u32,
  mldiff: u32,
  drdiff: u32,
  vwall: i16,
  viir: i16,
  vcomb1: i16,
  vcomb2: i16,
  vcomb3: i16,
  vcomb4: i16,
  mlcomb1: u32,
  mlcomb2: u32,
  mlcomb3: u32,
  mlcomb4: u32,
  mrcomb1: u32,
  mrcomb2: u32,
  mrcomb3: u32,
  mrcomb4: u32,
  dapf1: u32,
  dapf2: u32,
  vapf1: i16,
  vapf2: i16,
  mlapf1: u32,
  mlapf2: u32,
  mrapf1: u32,
  mrapf2: u32,

  far_input_l: VecDeque<i16>,
  far_input_r: VecDeque<i16>,
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
      reverb_start_address: 0,
      reverb_write_address: 0,
      reverb_output_volume_l: 0,
      reverb_output_volume_r: 0,
      reverb_input_volume_l: 0,
      reverb_input_volume_r: 0,
      reverb_left: true,
      mlsame: 0,
      dlsame: 0,
      mrsame: 0,
      drsame: 0,
      mrdiff: 0,
      dldiff: 0,
      mldiff: 0,
      drdiff: 0,
      vwall: 0,
      viir: 0,
      vcomb1: 0,
      vcomb2: 0,
      vcomb3: 0,
      vcomb4: 0,
      mlcomb1: 0,
      mlcomb2: 0,
      mlcomb3: 0,
      mlcomb4: 0,
      mrcomb1: 0,
      mrcomb2: 0,
      mrcomb3: 0,
      mrcomb4: 0,
      dapf1: 0,
      dapf2: 0,
      vapf1: 0,
      vapf2: 0,
      mlapf1: 0,
      mlapf2: 0,
      mrapf1: 0,
      mrapf2: 0,
      far_input_l: VecDeque::new(),
      far_input_r: VecDeque::new(),
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
        self.voices[index].store(index, offset % 0x10, val);
      }
      0x01A6 => { // 0x1F801DA6
        // サウンドRAMデータポート開始アドレス
        self.sound_ram_start_address = (val as u32) << 3;
        self.write_count = 0;
      }
      0x01A8 => { // 1F801DA8
        // サウンド RAM データ ポート (16 ビット)
        self.store16(self.sound_ram_start_address, val);
        self.sound_ram_start_address = self.sound_ram_start_address + 2;
        self.write_count = self.write_count + 1;
      }
      0x0188 => { // 0x1F801D88
        // キーオンボイス0～15（0=変更なし、1=キーオン）
        for (i, voice) in self.voices[0..15].iter_mut().enumerate() {
          let flag = (val & 0x01 << i) != 0;
          if flag {
            voice.key_on(&self.sound_ram);
          }
        }
      }
      0x018A => { // 0x1F801D8A
        // キーオンボイス16～23のキー
        for (i, voice) in self.voices[15..].iter_mut().enumerate() {
          let flag = (val & 0x01 << i) != 0;
          if flag {
            voice.key_on(&self.sound_ram);
          }
        }
      }
      0x018C => { // 0x1F801D8C
        // キーオフボイス 0-15 (0=変更なし、1=キーオフ)
        for (i, voice) in self.voices[0..15].iter_mut().enumerate() {
          let flag = (val & 0x01 << i) != 0;
          if flag {
            voice.key_off();
          }
        }
      }
      0x018E => { // 0x1F801D8E
        // キーオフボイス16-23
        for (i, voice) in self.voices[15..].iter_mut().enumerate() {
          let flag = (val & 0x01 << i) != 0;
          if flag {
            voice.key_off();
          }
        }
      }
      0x0198 => { // 1F801D98 ボイス0～15にリバーブが有効
        for (i, voice) in self.voices[0..15].iter_mut().enumerate() {
          let flag = (val & 0x01 << i) != 0;
          voice.reverb_enabled = flag;
        }
      }
      0x019A => { // 1F801D9A ボイス16～23にリバーブが有効
        for (i, voice) in self.voices[15..].iter_mut().enumerate() {
          let flag = (val & 0x01 << i) != 0;
          voice.reverb_enabled = flag;
        }
      }

      0x0180 => { // 0x1F801D80 メイン左ボリューム
        if val & 0x8000 != 0 {
          // エンベロープ有効
        } else {
          // 一定音量
          self.main_volume_l = (val << 1) as i16;
        }
      }
      0x0182 => { // 0x1F801D82 メイン右ボリューム
        if val & 0x8000 != 0 {
          // エンベロープ有効
        } else {
          // 一定音量
          self.main_volume_r = (val << 1) as i16;
        }
      }
      0x0184 => { // 1F801D84h spu   vLOUT   volume  Reverb Output Volume Left
        self.reverb_output_volume_l = val as i16;
      }
      0x0186 => { // 1F801D86h spu   vROUT   volume  Reverb Output Volume Right
        self.reverb_output_volume_r = val as i16;
      }
      0x01A2 => { // 1F801DA2 mBASE Reverb Work Area Start Address in Sound RAM
        self.reverb_start_address = (val as u32) << 3;
        self.reverb_write_address = self.reverb_start_address;
      }
      0x01FC => { // 0x1F801DFC リバーブ入力ボリューム左
        self.reverb_input_volume_l = val as i16;
      }
      0x01FE => { // 0x1F801DFE リバーブ入力ボリューム右
        self.reverb_input_volume_r = val as i16;
      }
      0x01D4 => { // 1F801DD4 mLSAME Reverb Same Side Reflection Address 1 Left
        self.mlsame = (val as u32) << 3;
      }
      0x01E0 => { // 1F801DE0 dLSAME Reverb Same Side Reflection Address 2 Left
        self.dlsame = (val as u32) << 3;
      }
      0x01D6 => { // 1F801DD6h rev0B mRSAME  src/dst Reverb Same Side Reflection Address 1 Right
        self.mrsame = (val as u32) << 3;
      }
      0x01E2 => { // 1F801DE2h rev11 dRSAME  src     Reverb Same Side Reflection Address 2 Right
        self.drsame = (val as u32) << 3;
      }
      0x01E6 => { // 1F801DE6h rev13 mRDIFF  src/dst Reverb Different Side Reflect Address 1 Right
        self.mrdiff = (val as u32) << 3;
      }
      0x01F0 => { // 1F801DF0h rev18 dLDIFF  src     Reverb Different Side Reflect Address 2 Left
        self.dldiff = (val as u32) << 3;
      }
      0x01E4 => { // 1F801DE4h rev12 mLDIFF  src/dst Reverb Different Side Reflect Address 1 Left
        self.mldiff = (val as u32) << 3;
      }
      0x01F2 => { // 1F801DF2h rev19 dRDIFF  src     Reverb Different Side Reflect Address 2 Right
        self.drdiff = (val as u32) << 3;
      }

      0x01C4 => { // 1F801DC4h rev02 vIIR    volume  Reverb Reflection Volume 1
        self.viir = val as i16;
      }
      0x01CE => { // 1F801DCEh rev07 vWALL   volume  Reverb Reflection Volume 2
        self.vwall = val as i16;
      }

      // COMB
      0x01C6 => { // 1F801DC6h rev03 vCOMB1  volume  Reverb Comb Volume 1
        self.vcomb1 = val as i16;
      }
      0x01C8 => { // 1F801DC8h rev04 vCOMB2  volume  Reverb Comb Volume 2
        self.vcomb2 = val as i16;
      }
      0x01CA => { // 1F801DCAh rev05 vCOMB3  volume  Reverb Comb Volume 3
        self.vcomb3 = val as i16;
      }
      0x01CC => { // 1F801DCCh rev06 vCOMB4  volume  Reverb Comb Volume 4
        self.vcomb4 = val as i16;
      }
      0x01D8 => { // 1F801DD8h rev0C mLCOMB1 src     Reverb Comb Address 1 Left
        self.mlcomb1 = (val as u32) << 3;
      }
      0x01DA => { // 1F801DDAh rev0D mRCOMB1 src     Reverb Comb Address 1 Right
        self.mrcomb1 = (val as u32) << 3;
      }
      0x01DC => { // 1F801DDCh rev0E mLCOMB2 src     Reverb Comb Address 2 Left
        self.mlcomb2 = (val as u32) << 3;
      }
      0x01DE => { // 1F801DDEh rev0F mRCOMB2 src     Reverb Comb Address 2 Right
        self.mrcomb2 = (val as u32) << 3;
      }
      0x01E8 => { // 1F801DE8h rev14 mLCOMB3 src     Reverb Comb Address 3 Left
        self.mlcomb3 = (val as u32) << 3;
      }
      0x01EA => { // 1F801DEAh rev15 mRCOMB3 src     Reverb Comb Address 3 Right
        self.mrcomb3 = (val as u32) << 3;
      }
      0x01EC => { // 1F801DECh rev16 mLCOMB4 src     Reverb Comb Address 4 Left
        self.mlcomb4 = (val as u32) << 3;
      }
      0x01EE => { // 1F801DEEh rev17 mRCOMB4 src     Reverb Comb Address 4 Right
        self.mrcomb4 = (val as u32) << 3;
      }

      // APF
      0x01C0 => { // 1F801DC0h rev00 dAPF1   disp    Reverb APF Offset 1
        self.dapf1 = (val as u32) << 3;
      }
      0x01C2 => { // 1F801DC2h rev01 dAPF2   disp    Reverb APF Offset 2
        self.dapf2 = (val as u32) << 3;
      }
      0x01D0 => { // 1F801DD0h rev08 vAPF1   volume  Reverb APF Volume 1
        self.vapf1 = val as i16;
      }
      0x01D2 => { // 1F801DD2h rev09 vAPF2   volume  Reverb APF Volume 2
        self.vapf2 = val as i16;
      }
      0x01F4 => { // 1F801DF4h rev1A mLAPF1  src/dst Reverb APF Address 1 Left
        self.mlapf1 = (val as u32) << 3;
      }
      0x01F6 => { // 1F801DF6h rev1B mRAPF1  src/dst Reverb APF Address 1 Right
        self.mrapf1 = (val as u32) << 3;
      }
      0x01F8 => { // 1F801DF8h rev1C mLAPF2  src/dst Reverb APF Address 2 Left
        self.mlapf2 = (val as u32) << 3;
      }
      0x01FA => { // 1F801DFAh rev1D mRAPF2  src/dst Reverb APF Address 2 Right
        self.mrapf2 = (val as u32) << 3;
      }
      _ => {
        // println!("Unhandled SPU store: {:08X}({:04X}) {:04X}", abs_addr, offset, val);
      }
    }
  }

  pub fn clock(&mut self) {

    let mut mixed_l = 0;
    let mut mixed_r = 0;
    let mut reverb: i32 = 0;
    for voice in &mut self.voices {
      if !voice.keyed_on {
        continue;
      }

      voice.clock(&self.sound_ram);

      let s = voice.current_sample;
      let (voice_sample_l, voice_sample_r) = voice.apply_voice_volume(s);
      mixed_l += i32::from(voice_sample_l);
      mixed_r += i32::from(voice_sample_r);

      if voice.reverb_enabled {
        reverb += i32::from(if self.reverb_left { voice_sample_l } else { voice_sample_r });
      }
    }

    let clamped_l = mixed_l.clamp(-0x8000, 0x7FFF) as i16;
    let clamped_r = mixed_r.clamp(-0x8000, 0x7FFF) as i16;

    let clamped_reverb = reverb.clamp(-0x8000, 0x7FFF) as i16;

    self.store16(self.reverb_write_address, clamped_reverb as u16);
    self.reverb_write_address = self.reverb_write_address.wrapping_add(2);
    if self.reverb_write_address > 0x7FFFF {
      self.reverb_write_address = self.reverb_start_address;
    }

    // reverb
    let input_sample = if self.reverb_left { clamped_l } else { clamped_r };
    // 同じ側のL
    self.apply_same_side_reflection(input_sample, self.mlsame, self.dlsame);
    // 同じ側のR
    self.apply_same_side_reflection(input_sample, self.mrsame, self.drsame);
    // // 異なる側LからR
    self.apply_same_side_reflection(input_sample, self.mrdiff, self.dldiff);
    // // 異なる側のRからL
    self.apply_same_side_reflection(input_sample, self.mldiff, self.drdiff);

    let comb_out = self.apply_comb_filter();
    let apf1_out = self.apply_all_pass_filter_1(comb_out);
    let apf2_out = self.apply_all_pass_filter_2(apf1_out);
    if self.reverb_left {
      push_input_sample(&mut self.far_input_l, apf2_out);
    } else {
      push_input_sample(&mut self.far_input_r, apf2_out);
    }

    let reverb_l = apply_fir_filter(&self.far_input_l);
    let reverb_r = apply_fir_filter(&self.far_input_r);

    self.reverb_left = !self.reverb_left;

    let with_reverb_l = clamped_l + reverb_l;
    let with_reverb_r = clamped_r + reverb_r;

    let output_l = apply_volume(with_reverb_l, self.main_volume_l);
    let output_r = apply_volume(with_reverb_r, self.main_volume_r);
    self.device.queue_audio(&[output_l, output_r]).unwrap()
  }

  fn reverb_relative_addr(&self, offset: u32) -> u32 {
    let addr = offset + self.reverb_write_address;
    let addr = addr % self.sound_ram.len() as u32;
    if addr < self.reverb_start_address {
      return self.reverb_start_address + addr
    }
    return addr
  }

  fn loadi16(&self, offset: u32) -> i16 {
    let offset = offset as usize;

    let b0 = self.sound_ram[offset + 0] as u16;
    let b1 = self.sound_ram[offset + 1] as u16;

    (b0 | (b1 << 8)) as i16
  }

  fn store16(&mut self, offset: u32, val: u16) {
    let offset = offset as usize;

    let b0 = val as u8;
    let b1 = (val >> 8) as u8;

    self.sound_ram[offset + 0] = b0;
    self.sound_ram[offset + 1] = b1;
  }

  fn apply_same_side_reflection(&mut self, input_sample: i16, m_addr: u32, d_addr: u32) {
    if m_addr == 0 {
      return
    }
    let val = ((input_sample as i32 + self.loadi16(self.reverb_relative_addr(d_addr)) as i32 * self.vwall as i32 - self.loadi16(self.reverb_relative_addr(m_addr - 2)) as i32) * self.viir as i32 + self.loadi16(self.reverb_relative_addr(m_addr - 2)) as i32) as i16;
    self.store16(m_addr, val as u16);
  }

  fn apply_comb_filter(&mut self) -> i16 {
    let comb1 = self.loadi16(if self.reverb_left { self.mlcomb1 } else { self.mrcomb1 });
    let comb2 = self.loadi16(if self.reverb_left { self.mlcomb2 } else { self.mrcomb2 });
    let comb3 = self.loadi16(if self.reverb_left { self.mlcomb3 } else { self.mrcomb3 });
    let comb4 = self.loadi16(if self.reverb_left { self.mlcomb4 } else { self.mrcomb4 });
    let out = self.vcomb1 * comb1 + self.vcomb2 * comb2 + self.vcomb3 * comb3 + self.vcomb4 * comb4;
    out
  }

  fn apply_all_pass_filter_1(&mut self, input_sample: i16) -> i16 {
    let mapf = if self.reverb_left { self.mlapf1 } else { self.mrapf1 };
    let buffered = input_sample - self.vapf1 * self.loadi16(mapf - self.dapf1);
    self.store16(mapf, buffered as u16);
    let apf_out = self.vapf1 * buffered + self.loadi16(mapf - self.dapf1);
    apf_out
  }

  fn apply_all_pass_filter_2(&mut self, input_sample: i16) -> i16 {
    let mapf = if self.reverb_left { self.mlapf2 } else { self.mrapf2 };
    let buffered = input_sample - self.vapf2 * self.loadi16(mapf - self.dapf2);
    self.store16(mapf, buffered as u16);
    let apf_out = self.vapf2 * buffered + self.loadi16(mapf - self.dapf2);
    apf_out
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

  enable_sweep_l: bool,
  enable_sweep_r: bool,
  sweep_l: u16,
  sweep_r: u16,
  // envelope関連
  adsr1: u16,
  adsr2: u16,

  reverb_enabled: bool,
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

      enable_sweep_l: false,
      enable_sweep_r: false,
      sweep_l: 0,
      sweep_r: 0,

      adsr1: 0,
      adsr2: 0,
      reverb_enabled: false,
    }
  }

  fn clock(&mut self, sound_ram: &[u8]) {
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
        shift = ((self.adsr1 & 0x00F0) >> 4) as u8;
        step = 0;
      }
      AdsrPhase::Sustain => {
        // 3-0   Sustain Level     (0..0Fh)  ;Level=(N+1)*800h
        self.envelope.sustain_level = ((self.adsr1 & 0x000F) + 1) * 0x0800;

        // 31    Sustain Mode      (0=Linear, 1=Exponential)
        // 30    Sustain Direction (0=Increase, 1=Decrease) (until Key OFF flag)
        // 29    Not used?         (should be zero)
        // 28-24 Sustain Shift     (0..1Fh = Fast..Slow)
        // 23-22 Sustain Step      (0..3 = "+7,+6,+5,+4" or "-8,-7,-6,-5") (inc/dec)
        rate = if self.adsr2 & 0x8000 == 0 { ChangeRate::Linear } else { ChangeRate::Exponential };
        direction = if self.adsr2 & 0x4000 == 0 { Direction::Increasing } else { Direction::Decreasing };
        shift = ((self.adsr2 & 0x1F00) >> 8) as u8;
        step = ((self.adsr2 & 0x00C0) >> 6) as u8;
      }
      AdsrPhase::Release => {
        // 21    Release Mode      (0=Linear, 1=Exponential)
        // -     Release Direction (Fixed, always Decrease) (until Level 0000h)
        // 20-16 Release Shift     (0..1Fh = Fast..Slow)
        // -     Release Step      (Fixed, always "-8")
        rate = if (self.adsr2 & 0x0020) != 0 { ChangeRate::Linear } else { ChangeRate::Exponential };
        direction = Direction::Decreasing;
        shift = (self.adsr2 & 0x001F) as u8;
        step = 0;
      }
    }
    // self.envelope.check_for_phase_transition();
    // self.envelope.update(direction, rate, shift, step);
    self.envelope.clock(direction, rate, shift, step);

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

  fn store(&mut self, index: usize, offset: u32, val: u16) {
    match offset { /* 0x00 ~ 0x0F */
      0x00 => {
        // 左音量
        self.enable_sweep_l = val & 0x8000 != 0;
        if self.enable_sweep_l {
          self.sweep_l = val;
        } else {
          self.volume_l = (val << 1) as i16;
        }
      }
      0x02 => {
        // 右音量
        self.enable_sweep_r = val & 0x8000 != 0;
        if self.enable_sweep_r {
          self.sweep_r = val;
        } else {
          self.volume_r = (val << 1) as i16;
        }
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
  fn clock(&mut self, direction: Direction, rate: ChangeRate, shift: u8, step: u8) {
    let mut counter_decrement = ENVELOPE_CONTER_MAX >> shift.saturating_sub(11);

    if direction == Direction::Increasing && rate == ChangeRate::Exponential && self.level > 0x6000 {
      counter_decrement >>= 2;
    }

    self.counter = self.counter.saturating_sub(counter_decrement);
    if self.counter == 0 {
      self.counter = ENVELOPE_CONTER_MAX;
      self.update(direction, rate, shift, step);
      self.check_for_phase_transition();
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

const FIR_FILTER: &[i16; 39] = &[
  -0x0001, 0x0000, 0x0002, 0x0000, -0x000A, 0x0000, 0x0023, 0x0000,
  -0x0067, 0x0000, 0x010A, 0x0000, -0x0268, 0x0000, 0x0534, 0x0000,
  -0x0B90, 0x0000, 0x2806, 0x4000, 0x2806, 0x0000, -0x0B90, 0x0000,
   0x0534, 0x0000, -0x0268, 0x0000, 0x010A, 0x0000, -0x0067, 0x0000,
   0x0023, 0x0000, -0x000A, 0x0000, 0x0002, 0x0000, -0x0001,
];

fn push_input_sample(deque: &mut VecDeque<i16>, sample: i16) {
  if deque.len() == FIR_FILTER.len() {
    deque.pop_front();
  }
  deque.push_back(sample);
}

fn apply_fir_filter(deque: &VecDeque<i16>) -> i16 {
  FIR_FILTER.iter().zip(deque)
    .map(|(&a, &b)| ((a as i32 * b as i32) >> 15) as i16)
    .sum()
}
