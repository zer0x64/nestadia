use super::opcode::Opcode;
use std::convert::TryFrom as _;

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
    fn required_bytes(&self) -> usize {
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
                    pc + (offset as u16)
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

pub fn disassemble(mem: &[u8], start: u16) -> Vec<(u16, String)> {
    let mut index: usize = 0;
    let mut disassembly = Vec::new();

    while index < mem.len() {
        let mut disas = String::new();
        if let Ok(opcode) = Opcode::try_from(mem[index]) {
            disas += &format!("{:?}", &opcode)[..3].to_lowercase();

            let required_bytes = opcode.addressing_mode().required_bytes();

            if required_bytes < 1 {
                disassembly.push((start + (index as u16), disas));
                index += 1;
            } else if required_bytes < (mem.len() - index) {
                disas += " ";
                disas += &opcode
                    .addressing_mode()
                    .format(&mem[index + 1..], start + (index as u16));
                disassembly.push((start + (index as u16), disas));
                index += 1;
                index += required_bytes;
            } else {
                index += 1;
            }
        } else {
            disassembly.push((start + (index as u16), "???".to_string()));
            index += 1;
        }
    }

    disassembly
}

fn to_u16(data: &[u8]) -> u16 {
    (data[0] as u16) | ((data[1] as u16) << 8)
}
