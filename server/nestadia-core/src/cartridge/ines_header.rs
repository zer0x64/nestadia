use bitflags::bitflags;
use std::convert::TryFrom;

use crate::RomParserError;

const MAGIC_BYTES: [u8; 4] = [0x4e, 0x45, 0x53, 0x1a];

#[derive(Debug)]
pub struct INesHeader {
    pub mapper_id: u8,
    pub prg_size: u8,
    pub chr_size: u8,
    pub flags6: Flags6,
    pub flags7: Flags7,
    pub flags8: u8, // Flags 8 is actually the PRG ram size
    pub flags9: Flags9,
    pub flags10: Flags10,
}

bitflags! {
    pub struct Flags6: u8 {
        const MIRRORING = (1 << 0);
        const PRG_RAM = (1 << 1);
        const TRAINER = (1 << 2);
        const IGNORE_MIRRORING_CONTROL = (1 << 3);
    }
}

bitflags! {
    pub struct Flags7: u8 {
        const VS_UNISYSTEM = (1 << 0);
        const PLAYCHOICE_10 = (1 << 1);
        const NES2 = (1 << 2) | (1 << 3);
    }
}

bitflags! {
    pub struct Flags9: u8 {
        const TV_SYSTEM = (1 << 0);
    }
}

bitflags! {
    pub struct Flags10: u8 {
        const PAL = (2 << 0);
        const DUAL = (1 << 0);
        const PRG_RAM = (1 << 4);
        const BUS_CONFLICT = (1 << 5);
    }
}

impl TryFrom<&[u8]> for INesHeader {
    type Error = RomParserError;

    fn try_from(data: &[u8]) -> Result<Self, Self::Error> {
        if data.len() < 16 {
            return Err(RomParserError::TooShort);
        };

        if data[..4] != MAGIC_BYTES {
            return Err(RomParserError::InvalidMagicBytes);
        };

        let prg_size = data[4];
        let chr_size = data[5];

        let mapper_id = (data[6] >> 4) | (data[7] & 0xf0);

        let flags6 = Flags6::from_bits_truncate(data[6]);
        let flags7 = Flags7::from_bits_truncate(data[7]);
        let flags8 = data[8];
        let flags9 = Flags9::from_bits_truncate(data[9]);
        let flags10 = Flags10::from_bits_truncate(data[10]);

        return Ok(INesHeader {
            mapper_id,
            prg_size,
            chr_size,
            flags6,
            flags7,
            flags8,
            flags9,
            flags10,
        });
    }
}
