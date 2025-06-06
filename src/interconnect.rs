use core::panic;

use crate::{bios::Bios, channel::{Direction, Step, Sync}, dma::{Dma, Port}, gpu::Gpu, ram::Ram, spu::Spu};


pub struct Interconnect {
  bios: Bios,
  ram: Ram,
  dma: Dma,
  pub gpu: Gpu,
  pub spu: Spu,
}

impl Interconnect {
  pub fn new(bios: Bios, gpu: Gpu, spu: Spu) -> Self {
    Self {
      bios,
      ram: Ram::new(),
      dma: Dma::new(),
      gpu,
      spu,
    }
  }

  pub fn load32(&self, addr: u32) -> u32 {
    if addr % 4 != 0 {
      panic!("Unalignd load32 address: {:08X}", addr);
    }
    let abs_addr = mask_region(addr);

    if let Some(offset) = map::BIOS.contains(abs_addr) {
      return self.bios.load32(offset);
    }
    if let Some(offset) = map::RAM.contains(abs_addr) {
      return self.ram.load32(offset);
    }
    if let Some(offset) = map::IRQ_CONTROL.contains(abs_addr) {
      // println!("IRQ control read: {:X}", offset);
      return 0;
    }
    if let Some(offset) = map::DMA.contains(abs_addr) {
      return self.dma_reg(offset);
    }
    if let Some(offset) = map::GPU.contains(abs_addr) {
      return match offset {
        0 => self.gpu.read(),
        4 => self.gpu.status(),
        _ => panic!("GPU read: {:08X}", offset),
      };
    }
    if let Some(offset) = map::TIMERS.contains(abs_addr) {
      // println!("Unhandled write to timer register {:08X}", offset);
      return 0;
    }

    panic!("unhandled load32 at address {:08X}", addr);
  }

  pub fn store32(&mut self, addr: u32, val: u32) {
    if addr % 4 != 0 {
      panic!("Unalignd store32 address: {:08X}", addr);
    }
    let abs_addr = mask_region(addr);

    if let Some(offset) = map::MEM_CONTROL.contains(abs_addr) {
      match offset {
        0 => if val != 0x1F00_0000 {
          panic!("Bad expansion 1 base address: 0x{:01X}", val);
        }
        4 => if val != 0x1F80_2000 {
          panic!("Bad expansion 2 base address: 0x{:01X}", val);
        }
        _ => println!("Unhandled write to MEM_CONTROL register"),
      }
      return;
    }
    if let Some(_) = map::RAM_SIZE.contains(abs_addr) {
      println!("write to RAM_SIZE register {:08X}", abs_addr);
      return;
    }
    if let Some(_) = map::CACHE_CONTROL.contains(abs_addr) {
      println!("write to CACHE_CONTROL register {:08X}", abs_addr);
      return;
    }
    if let Some(offset) = map::RAM.contains(abs_addr) {
      self.ram.store32(offset, val);
      return;
    }
    if let Some(offset) = map::IRQ_CONTROL.contains(abs_addr) {
      // println!("IRQ control: {:X} <- {:08X}", offset, val);
      return;
    }
    if let Some(offset) = map::DMA.contains(abs_addr) {
      return self.set_dma_reg(offset, val);
    }
    if let Some(offset) = map::GPU.contains(abs_addr) {
      match offset {
        0 => self.gpu.gp0(val),
        4 => self.gpu.gp1(val),
        _ => panic!("GPU write: {:08X} {:08X}", offset, val),
      }
      return;
    }
    if let Some(offset) = map::TIMERS.contains(abs_addr) {
      // println!("Unhandled write to timer register {:08X} {:08X}", offset, val);
      return;
    }
    panic!("unhandled store32 at address {:08X}", addr)
  }

  pub fn load16(&self, addr: u32) -> u16 {
    if addr % 2 != 0 {
      panic!("Unalignd load16 address: {:08X}", addr);
    }
    let abs_addr = mask_region(addr);

    if let Some(offset) = map::SPU.contains(abs_addr) {
      // println!("Unhandled read from SPU register {:08X}", abs_addr);
      return self.spu.load(abs_addr, offset);
    }
    if let Some(offset) = map::RAM.contains(abs_addr) {
      return self.ram.load16(offset);
    }

    if let Some(offset) = map::IRQ_CONTROL.contains(abs_addr) {
      // println!("IRQ control read {:08X}", offset);
      return 0;
    }

    panic!("unhandled load16 at address {:08X}", addr);
  }

  pub fn store16(&mut self, addr: u32, val: u16) {
    if addr % 2 != 0 {
      panic!("Unalignd store16 address: {:08X}", addr);
    }

    let abs_addr = mask_region(addr);

    if let Some(offset) = map::RAM.contains(abs_addr) {
      return self.ram.store16(offset, val);
    }

    if let Some(offset) = map::SPU.contains(abs_addr) {
      // println!("Unhandled write to SPU register {:X}", offset);
      self.spu.store(abs_addr, offset, val);
      return;
    }

    if let Some(offset) = map::TIMERS.contains(abs_addr) {
      // println!("Unhandled write to timer register {:X}", offset);
      return;
    }

    if let Some(offset) = map::IRQ_CONTROL.contains(abs_addr) {
      // println!("IRQ control write {:08X} {:04X}", offset, val);
      return;
    }

    panic!("Unhandled store16 at address {:08X}", addr)
  }


  pub fn store8(&mut self, addr: u32, val: u8) {
    let abs_addr = mask_region(addr);

    if let Some(offset) = map::EXPANTION_2.contains(abs_addr) {
      println!("Unhandled write to EXPANTION_2 register {:X}", offset);
      return;
    }

    if let Some(offset) = map::RAM.contains(abs_addr) {
      self.ram.store8(offset, val);
      return;
    }

    panic!("Unhandled store8 at address {:08X}", addr)
  }

  pub fn load8(&self, addr: u32) -> u8 {
    let abs_addr = mask_region(addr);

    if let Some(offset) = map::BIOS.contains(abs_addr) {
      return self.bios.load8(offset);
    }

    if let Some(offset) = map::RAM.contains(abs_addr) {
      return self.ram.load8(offset);
    }

    if let Some(offset) = map::EXPANTION_1.contains(abs_addr) {
      return 0xFF; // No expantion implemented
    }

    panic!("Unhandled load8 at address {:08X}", addr);
  }

  fn dma_reg(&self, offset: u32) -> u32 {
    let major = (offset & 0x70) >> 4;
    let minor = offset & 0x0F;

    match major {
      0..=6 => {
        let channel = self.dma.channel(Port::from_index(major));

        match minor {
          8 => channel.control(),
          _ => panic!("Unhandled DMA read at {:08X}", offset)
        }
      },

      7 => match minor {
        0 => self.dma.control(),
        4 => self.dma.interrupt(),
        _ => panic!("Unhandled DMA read at {:08X}", offset)
      }
      _ => panic!("Unhandled DMA read at {:08X}", offset)
    }
  }

  fn set_dma_reg(&mut self, offset: u32, val: u32) {
    let major = (offset & 0x70) >> 4;
    let minor = offset & 0x0F;

    let active_port = match major {
      0..=6 => {
        let port = Port::from_index(major);
        let channel = self.dma.channel_mut(port);

        match minor {
          0 => channel.set_base(val),
          4 => channel.set_block_control(val),
          8 => channel.set_control(val),
          _ => panic!("Unhandled DMA write at {:08X}: {:08X}", offset, val)
        }

        if channel.active() {
          Some(port)
        } else {
          None
        }
      },

      7 => {
        match minor {
          0 => self.dma.set_control(val),
          4 => self.dma.set_interrupt(val),
          _ => panic!("Unhandled DMA write at {:08X}: {:08X}", offset, val)
        }
        None
      }
      _ => panic!("Unhandled DMA write at {:08X}: {:08X}", offset, val)
    };

    if let Some(port) = active_port {
      self.do_dma(port);
    }
  }

  fn do_dma(&mut self, port: Port) {
    match self.dma.channel(port).sync() {
      Sync::LinkedList => self.do_dma_linked_list(port),
      _ => self.do_dma_block(port),
    }
  }

  fn do_dma_block(&mut self, port: Port) {
    let channel = self.dma.channel_mut(port);
    let increment = channel.step();
    let mut addr = channel.base();
    let mut remsz = match channel.transfer_size() {
      Some(n) => n,
      None => panic!("Couldn't figure out DMA block transfer size"),
    };
    while remsz > 0 {
      let cur_addr = addr & 0x001F_FFFC;
      match channel.direction() {
        Direction::FromRam => {
          let src_word = self.ram.load32(cur_addr);
          match port {
            Port::Gpu => self.gpu.gp0(src_word),
            _ => panic!("Unhandled DMA destination port {}", port as u8),
          }
        },
        Direction::ToRam => {
          let src_word = match port {
            Port::Otc => match remsz {
              1 => 0x00FF_FFFF,
              _ => addr.wrapping_sub(4) & 0x001F_FFFF,
            },
            _ => panic!("Unhandled DMA source port: {}", port as u8),
          };
          self.ram.store32(cur_addr, src_word);
        }
      }

      addr = match increment {
        Step::Increment => addr.wrapping_add(4),
        Step::Decrement => addr.wrapping_sub(4)
      };
      remsz = remsz - 1;
    }
    channel.done();
  }

  fn do_dma_linked_list(&mut self, port: Port) {
    let channel = self.dma.channel_mut(port);
    let mut addr = channel.base() & 0x001F_FFFC;
    if channel.direction() == Direction::ToRam {
      panic!("Invalid DMA direction for linked list mode");
    }
    if port != Port::Gpu {
      panic!("Attempt linked list DMA on port {}", port as u8);
    }

    loop {
      let header = self.ram.load32(addr);
      let mut remsz = header >> 24;
      while remsz > 0 {
        addr = addr.wrapping_add(4) & 0x001F_FFFC;
        let command = self.ram.load32(addr);
        self.gpu.gp0(command);
        remsz = remsz - 1;
      }

      if header & 0x0080_0000 != 0 {
        break;
      }

      addr = header & 0x001F_FFFC;
    }
    channel.done();
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

  pub const RAM: Range = Range(0x0000_0000, 2 * 1024 * 1024);
  pub const BIOS: Range = Range(0x1FC0_0000, 512 * 1024);
  pub const MEM_CONTROL: Range = Range(0x1F80_1000, 36); // SYS_CONTROL
  pub const RAM_SIZE: Range = Range(0x1F80_1060, 4);
  pub const CACHE_CONTROL: Range = Range(0xFFFE_0130, 4);
  pub const SPU: Range = Range(0x1F80_1C00, 640);
  pub const EXPANTION_2: Range = Range(0x1F80_2000, 66);
  pub const EXPANTION_1: Range = Range(0x1F00_0000, 512 * 1024);
  pub const IRQ_CONTROL: Range = Range(0x1F80_1070, 8);
  pub const TIMERS: Range = Range(0x1F80_1100, 16 * 3);
  pub const DMA: Range = Range(0x1F80_1080, 0x80);
  pub const GPU: Range = Range(0x1F80_1810, 8); // GP0, GP1
}

const REGION_MASK: [u32; 8] = [
  // KUSEG: 2048KB
  0xFFFF_FFFF, 0xFFFF_FFFF, 0xFFFF_FFFF, 0xFFFF_FFFF,
  // KSEG0: 512KB
  0x7FFF_FFFF,
  // KSEG1: 512KB
  0x1FFF_FFFF,
  // KSEG2: 1024KB
  0xFFFF_FFFF, 0xFFFF_FFFF,
];

pub fn mask_region(addr: u32) -> u32 {
  let index = (addr >> 29) as usize;
  addr & REGION_MASK[index]
}
