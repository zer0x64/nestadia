mod ines_header;
mod mapper_nrom;

use crate::RomParserError;

use ines_header::{Flags6, INesHeader};
use mapper_nrom::NRom;

use std::convert::TryFrom as _;

use log;

const PRG_BANK_SIZE: usize = 16384;
const CHR_BANK_SIZE: usize = 8192;

trait Mapper: Send + Sync {
    fn cpu_map_read(&self, addr: u16) -> u16;
    fn cpu_map_write(&self, addr: u16) -> Option<u16>;
    fn ppu_map_read(&self, addr: u16) -> u16;
    fn ppu_map_write(&self, addr: u16) -> Option<u16>;
}

pub struct Cartridge {
    header: INesHeader,

    prg_memory: Vec<u8>,
    chr_memory: Vec<u8>,
    mapper: Box<dyn Mapper>,
}

impl Cartridge {
    pub fn new(rom: &[u8]) -> Result<Self, RomParserError> {
        let header: INesHeader = INesHeader::try_from(rom)?;

        log::info!("ROM info: {:?}", &header);

        let mut prg_memory = vec![0u8; PRG_BANK_SIZE * (header.prg_size as usize)];
        let mut chr_memory = vec![0u8; CHR_BANK_SIZE * (header.chr_size as usize)];

        let mapper = match header.mapper_id {
            0 => Box::new(NRom::new(header.prg_size)),
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

    pub fn cpu_read(&self, addr: u16) -> u8 {
        let addr = self.mapper.cpu_map_read(addr);
        self.prg_memory[addr as usize]
    }

    pub fn cpu_write(&mut self, addr: u16, data: u8) {
        let addr = self.mapper.cpu_map_write(addr);

        if let Some(addr) = addr {
            self.prg_memory[addr as usize] = data;
        }
    }

    pub fn ppu_read(&self, addr: u16) -> u8 {
        let addr = self.mapper.ppu_map_read(addr);
        self.chr_memory[addr as usize]
    }

    pub fn ppu_write(&mut self, addr: u16, data: u8) {
        let addr = self.mapper.ppu_map_write(addr);
        
        if let Some(addr) = addr {
            self.chr_memory[addr as usize] = data;
        }
    }

    pub fn disassemble(&self) -> Vec<(u16, String)> {
        let mut disas1 = crate::cpu::disassembler::disassemble(&self.prg_memory, 0x4000);
        let disas2 = crate::cpu::disassembler::disassemble(&self.prg_memory, 0x8000);
        let disas3 = crate::cpu::disassembler::disassemble(&self.prg_memory, 0xc000);

        disas1.extend(disas2);
        disas1.extend(disas3);
        disas1
    }
}
