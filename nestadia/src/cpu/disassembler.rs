use super::opcode::Opcode;
use alloc::format;
use alloc::string::String;
use alloc::string::ToString;
use alloc::vec::Vec;
use core::convert::TryFrom as _;

pub enum AddressingMode {
    Accumulator,
    Immediate,
    Implied,
    Relative,
    Absolute,
    AbsoluteX,
    AbsoluteY,
    ZeroPage,
    ZeroPageX,
    ZeroPageY,
    Indirect,
    IndirectX,
    IndirectY,
}

impl AddressingMode {
    fn required_bytes(&self) -> u16 {
        match &self {
            AddressingMode::Accumulator => 0,
            AddressingMode::Immediate => 1,
            AddressingMode::Implied => 0,
            AddressingMode::Relative => 1,
            AddressingMode::Absolute => 2,
            AddressingMode::AbsoluteX => 2,
            AddressingMode::AbsoluteY => 2,
            AddressingMode::ZeroPage => 1,
            AddressingMode::ZeroPageX => 1,
            AddressingMode::ZeroPageY => 1,
            AddressingMode::Indirect => 2,
            AddressingMode::IndirectX => 1,
            AddressingMode::IndirectY => 1,
        }
    }

    fn format(&self, data: &[u8], pc: u16) -> String {
        match &self {
            AddressingMode::Accumulator => "a".to_string(),
            AddressingMode::Immediate => format!("#{:#x}", data[0]),
            AddressingMode::Implied => String::new(),
            AddressingMode::Relative => {
                let offset = data[0];
                let address = if offset <= 0x80 {
                    pc.wrapping_add(offset as u16)
                } else {
                    pc - (0xff - offset as u16) + 1
                };

                format!("{:#x}", address)
            }
            AddressingMode::Absolute => format!("{:#x}", to_u16(&data[..2])),
            AddressingMode::AbsoluteX => format!("{:#x},x", to_u16(&data[..2])),
            AddressingMode::AbsoluteY => format!("{:#x},y", to_u16(&data[..2])),
            AddressingMode::ZeroPage => format!("{:#x}", data[0]),
            AddressingMode::ZeroPageX => format!("{:#x},x", data[0]),
            AddressingMode::ZeroPageY => format!("{:#x},y", data[0]),
            AddressingMode::Indirect => format!("({:#x})", to_u16(&data[..2])),
            AddressingMode::IndirectX => format!("({:#x},x)", data[0]),
            AddressingMode::IndirectY => format!("({:#x}),y", data[0]),
        }
    }
}

pub fn disassemble(
    cart: &crate::cartridge::Cartridge,
    start: u16,
) -> Vec<(Option<u8>, u16, String)> {
    let mut addr: u16 = start;
    let mut disassembly = Vec::new();

    while addr < 0xFFFF {
        let mut disas = String::new();
        let prg_bank = cart.get_prg_bank(addr);
        if let Ok(opcode) = Opcode::try_from(cart.read_prg_mem(addr)) {
            disas += &format!("{:?}", &opcode)[..3].to_lowercase();

            let required_bytes = opcode.addressing_mode().required_bytes();

            if required_bytes < 1 {
                disassembly.push((prg_bank, addr, disas));
                addr += 1;
            } else if required_bytes < (0xFFFF - addr) {
                let data = (0..required_bytes)
                    .map(|i| cart.read_prg_mem(addr + i + 1))
                    .collect::<Vec<_>>();

                disas += " ";
                disas += &opcode
                    .addressing_mode()
                    .format(data.as_slice(), addr + required_bytes + 1);
                disassembly.push((prg_bank, addr, disas));
                addr += required_bytes + 1;
            } else {
                addr += 1;
            }
        } else {
            disassembly.push((prg_bank, addr, "???".to_string()));
            addr += 1;
        }
    }

    disassembly
}

fn to_u16(data: &[u8]) -> u16 {
    (data[0] as u16) | ((data[1] as u16) << 8)
}
