mod ines_header;
mod mapper_000;
mod mapper_002;
mod mapper_003;
mod mapper_066;

use std::convert::TryFrom as _;

use self::ines_header::{Flags6, INesHeader};
use self::mapper_000::Mapper000;
use self::mapper_002::Mapper002;
use self::mapper_003::Mapper003;
use self::mapper_066::Mapper066;

#[derive(Debug, Clone, Copy)]
pub enum Mirroring {
    Horizontal,
    Vertical,
    FourScreen,
}

#[derive(Debug, Clone, Copy)]
pub enum RomParserError {
    TooShort,
    InvalidMagicBytes,
    MapperNotImplemented,
}

impl std::fmt::Display for RomParserError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{:?}", &self)
    }
}

impl std::error::Error for RomParserError {}

trait Mapper: Send + Sync {
    fn cpu_map_read(&self, addr: u16) -> u16;
    fn cpu_map_write(&mut self, addr: u16, data: u8);
    fn ppu_map_read(&self, addr: u16) -> u16;
    fn ppu_map_write(&self, addr: u16) -> Option<u16>;
}

pub struct Cartridge {
    header: INesHeader,
    prg_memory: Vec<u8>, // program ROM, used by CPU
    chr_memory: Vec<u8>, // character ROM, used by PPU
    mapper: Box<dyn Mapper>,
}

impl Cartridge {
    pub fn load(rom: &[u8]) -> Result<Self, RomParserError> {
        const PRG_BANK_SIZE: usize = 16384;
        const CHR_BANK_SIZE: usize = 8192;

        let header: INesHeader = INesHeader::try_from(rom)?;

        log::info!("ROM info: {:?}", &header);

        let mut prg_memory = vec![0u8; PRG_BANK_SIZE * (header.prg_size as usize)];
        let mut chr_memory = vec![0u8; CHR_BANK_SIZE * (header.chr_size as usize)];

        let mapper: Box<dyn Mapper> = match header.mapper_id {
            0 => Box::new(Mapper000::new(header.prg_size)),
            2 => Box::new(Mapper002::new(header.prg_size)),
            3 => Box::new(Mapper003::new(header.prg_size)),
            66 => Box::new(Mapper066::new()),
            _ => return Err(RomParserError::MapperNotImplemented),
        };

        let mut start: usize = if header.flags6.contains(Flags6::TRAINER) {
            512 + 16
        } else {
            16
        };

        let end = start + prg_memory.len();

        prg_memory.copy_from_slice(&rom[start..end]);

        start += prg_memory.len();
        let end = start + chr_memory.len();
        chr_memory.copy_from_slice(&rom[start..end]);

        Ok(Cartridge {
            header,
            prg_memory,
            chr_memory,
            mapper,
        })
    }

    pub fn mirroring(&self) -> Mirroring {
        if self.header.flags6.contains(Flags6::FOUR_SCREEN) {
            Mirroring::FourScreen
        } else if self.header.flags6.contains(Flags6::MIRRORING) {
            Mirroring::Vertical
        } else {
            Mirroring::Horizontal
        }
    }

    pub fn read_prg_mem(&self, addr: u16) -> u8 {
        let addr = self.mapper.cpu_map_read(addr);
        self.prg_memory[addr as usize]
    }

    pub fn write_prg_mem(&mut self, addr: u16, data: u8) {
        self.mapper.cpu_map_write(addr, data);
    }

    pub fn read_chr_mem(&self, addr: u16) -> u8 {
        let addr = self.mapper.ppu_map_read(addr) as usize;
        if addr < self.chr_memory.len() {
            self.chr_memory[addr as usize]
        } else {
            0
        }
    }

    pub fn write_chr_mem(&mut self, addr: u16, data: u8) {
        if let Some(addr) = self.mapper.ppu_map_write(addr) {
            self.chr_memory[addr as usize] = data;
        } else {
            log::info!(
                "attempted to write on CHR memory at {}, but this is not supported by this mapper",
                addr
            );
        }
    }

    #[cfg(feature = "debugger")]
    pub fn disassemble(&self) -> Vec<(u16, String)> {
        let mut disas1 = crate::cpu::disassembler::disassemble(&self.prg_memory, 0x4000);
        let disas2 = crate::cpu::disassembler::disassemble(&self.prg_memory, 0x8000);
        let disas3 = crate::cpu::disassembler::disassemble(&self.prg_memory, 0xc000);

        disas1.extend(disas2);
        disas1.extend(disas3);
        disas1
    }
}
