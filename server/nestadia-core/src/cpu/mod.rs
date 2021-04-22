mod opcode;
pub mod disassembler;

use bitflags::bitflags;
use log;
use std::convert::TryFrom as _;

use crate::EmulatorContext;
use opcode::Opcode;

const STACK_BASE: u16 = 0x100;
const PC_START: u16 = 0xfffc;
const IRQ_HANDLER: u16 = 0xfffe;
const NMI_HANDLER: u16 = 0xfffa;

bitflags! {
    pub struct StatusRegister: u8 {
        const C = (1 << 0);
        const Z = (1 << 1);
        const I = (1 << 2);
        const D = (1 << 3);
        const B = (1 << 4);
        const U = (1 << 5);
        const V = (1 << 6);
        const N = (1 << 7);
    }
}

#[derive(Clone, Debug)]
pub struct Cpu {
    pub a: u8,
    pub x: u8,
    pub y: u8,
    pub st: u8,
    pub pc: u16,
    pub cycles: u8,
    pub status_register: StatusRegister,
}

impl Cpu {
    pub fn new() -> Self {
        Self {
            a: 0,
            x: 0,
            y: 0,
            st: 0,
            pc: 0,
            cycles: 0,
            status_register: StatusRegister::empty(),
        }
    }
}

impl dyn EmulatorContext<Cpu> {
    pub fn reset(&mut self) {
        self.a = 0;
        self.x = 0;
        self.y = 0;
        self.st = 0xFD;
        self.cycles = 8;
        self.status_register = StatusRegister::empty() | StatusRegister::U;

        self.pc = self.cpu_read(PC_START, false) as u16
            | ((self.cpu_read(PC_START.wrapping_add(1), false) as u16) << 8);
    }

    pub fn irq(&mut self) {
        if !self.status_register.contains(StatusRegister::I) {
            // Push current PC
            self.stack_push(((self.pc >> 8) & 0xff) as u8);
            self.stack_push((self.pc & 0xff) as u8);

            // Push status register
            self.status_register.set(StatusRegister::B, false);
            self.status_register.set(StatusRegister::U, true);
            self.stack_push(self.status_register.bits());

            self.status_register.set(StatusRegister::I, true);

            self.pc = self.cpu_read(IRQ_HANDLER, false) as u16
                | ((self.cpu_read(IRQ_HANDLER.wrapping_add(1), false) as u16) << 8);

            self.cycles = 7;
        }
    }

    pub fn nmi(&mut self) {
        // Push current PC
        self.stack_push(((self.pc >> 8) & 0xff) as u8);
        self.stack_push((self.pc & 0xff) as u8);

        // Push status register
        self.status_register.set(StatusRegister::B, false);
        self.status_register.set(StatusRegister::U, true);
        self.stack_push(self.status_register.bits());

        self.status_register.set(StatusRegister::I, true);

        self.pc = self.cpu_read(NMI_HANDLER, false) as u16
            | ((self.cpu_read(NMI_HANDLER.wrapping_add(1), false) as u16) << 8);

        self.cycles = 7;
    }

    pub fn clock(&mut self) {
        if self.cycles == 0 {
            let opcode = match Opcode::try_from(self.cpu_read(self.pc, false)) {
                Ok(o) => o,
                Err(_) => {
                    log::warn!(
                        "Unknown opcode {}, treating as a NOP...",
                        self.cpu_read(self.pc, false)
                    );
                    Opcode::Nop
                }
            };
            self.pc = self.pc.wrapping_add(1);

            // TODO: Remove log
            log::info!("{:x?} {:x?}", opcode, self.get_ref());

            match &opcode {
                Opcode::Brk => {
                    self.inst_brk();
                }
                Opcode::OraIndX => {
                    let addr = self.am_izx();
                    let op = self.cpu_read(addr, false);
                    self.inst_ora(op)
                }
                Opcode::OraZp => {
                    let addr = self.am_zp();
                    let op = self.cpu_read(addr, false);
                    self.inst_ora(op)
                }
                Opcode::AslZp => {
                    let addr = self.am_zp();
                    let op = self.cpu_read(addr, false);
                    let result = self.inst_asl(op);
                    self.cpu_write(addr, result);
                }
                Opcode::Php => {
                    self.inst_php();
                }
                Opcode::OraImm => {
                    let op = self.am_imm();
                    self.inst_ora(op);
                }
                Opcode::AslAcc => {
                    self.a = self.inst_asl(self.a);
                }
                Opcode::OraAbs => {
                    let addr = self.am_abs();
                    let op = self.cpu_read(addr, false);
                    self.inst_ora(op);
                }
                Opcode::AslAbs => {
                    let addr = self.am_abs();
                    let op = self.cpu_read(addr, false);
                    let result = self.inst_asl(op);
                    self.cpu_write(addr, result);
                }

                Opcode::Bpl => {
                    let addr = self.am_rel();
                    self.inst_bpl(addr);
                }
                Opcode::OraIndY => {
                    let (addr, extra_cycle) = self.am_izy();
                    if extra_cycle {
                        self.cycles += 1;
                    }

                    let op = self.cpu_read(addr, false);
                    self.inst_ora(op);
                }
                Opcode::OraZpX => {
                    let addr = self.am_zpx();
                    let op = self.cpu_read(addr, false);
                    self.inst_ora(op);
                }
                Opcode::AslZpX => {
                    let addr = self.am_zpx();
                    let op = self.cpu_read(addr, false);
                    let result = self.inst_asl(op);
                    self.cpu_write(addr, result);
                }
                Opcode::Clc => {
                    self.inst_clc();
                }
                Opcode::OraAbsY => {
                    let (addr, extra_cycle) = self.am_aby();
                    if extra_cycle {
                        self.cycles += 1;
                    }

                    let op = self.cpu_read(addr, false);
                    self.inst_ora(op);
                }
                Opcode::OraAbsX => {
                    let (addr, extra_cycle) = self.am_abx();
                    if extra_cycle {
                        self.cycles += 1;
                    }

                    let op = self.cpu_read(addr, false);
                    self.inst_ora(op);
                }
                Opcode::AslAbsX => {
                    let (addr, _) = self.am_abx();
                    let op = self.cpu_read(addr, false);
                    let result = self.inst_asl(op);
                    self.cpu_write(addr, result);
                }

                Opcode::JsrAbs => {
                    let addr = self.am_abs();
                    self.inst_jsr(addr);
                }
                Opcode::AndIndX => {
                    let addr = self.am_izx();
                    let op = self.cpu_read(addr, false);
                    self.inst_and(op);
                }
                Opcode::BitZp => {
                    let addr = self.am_zp();
                    let op = self.cpu_read(addr, false);
                    self.inst_bit(op);
                }
                Opcode::AndZp => {
                    let addr = self.am_zp();
                    let op = self.cpu_read(addr, false);
                    self.inst_and(op);
                }
                Opcode::RolZp => {
                    let addr = self.am_zp();
                    let op = self.cpu_read(addr, false);
                    let result = self.inst_rol(op);
                    self.cpu_write(addr, result);
                }
                Opcode::Plp => {
                    self.inst_plp();
                }
                Opcode::AndImm => {
                    let op = self.am_imm();
                    self.inst_and(op);
                }
                Opcode::RolAcc => {
                    self.a = self.inst_rol(self.a);
                }
                Opcode::BitAbs => {
                    let addr = self.am_abs();
                    let op = self.cpu_read(addr, false);
                    self.inst_bit(op);
                }
                Opcode::AndAbs => {
                    let addr = self.am_abs();
                    let op = self.cpu_read(addr, false);
                    self.inst_and(op);
                }
                Opcode::RolAbs => {
                    let addr = self.am_abs();
                    let op = self.cpu_read(addr, false);
                    let result = self.inst_rol(op);
                    self.cpu_write(addr, result);
                }

                Opcode::Bmi => {
                    let addr = self.am_rel();
                    self.inst_bmi(addr);
                }
                Opcode::AndIndY => {
                    let (addr, extra_cycle) = self.am_izy();
                    if extra_cycle {
                        self.cycles += 1;
                    }

                    let op = self.cpu_read(addr, false);
                    self.inst_and(op);
                }
                Opcode::AndZpX => {
                    let addr = self.am_zpx();
                    let op = self.cpu_read(addr, false);
                    self.inst_and(op);
                }
                Opcode::RolZpX => {
                    let addr = self.am_zpx();
                    let op = self.cpu_read(addr, false);
                    let result = self.inst_rol(op);
                    self.cpu_write(addr, result);
                }
                Opcode::Sec => {
                    self.inst_sec();
                }
                Opcode::AndAbsY => {
                    let (addr, extra_cycle) = self.am_aby();
                    if extra_cycle {
                        self.cycles += 1;
                    }

                    let op = self.cpu_read(addr, false);
                    self.inst_and(op);
                }
                Opcode::AndAbsX => {
                    let (addr, extra_cycle) = self.am_abx();
                    if extra_cycle {
                        self.cycles += 1;
                    }

                    let op = self.cpu_read(addr, false);
                    self.inst_and(op);
                }
                Opcode::RolAbsX => {
                    let (addr, _) = self.am_abx();
                    let op = self.cpu_read(addr, false);
                    let result = self.inst_rol(op);
                    self.cpu_write(addr, result);
                }

                Opcode::Rti => {
                    self.inst_rti();
                }
                Opcode::EorIndX => {
                    let addr = self.am_izx();
                    let op = self.cpu_read(addr, false);
                    self.inst_eor(op);
                }
                Opcode::EorZp => {
                    let addr = self.am_zp();
                    let op = self.cpu_read(addr, false);
                    self.inst_eor(op);
                }
                Opcode::LsrZp => {
                    let addr = self.am_zp();
                    let op = self.cpu_read(addr, false);
                    let result = self.inst_lsr(op);
                    self.cpu_write(addr, result);
                }
                Opcode::Pha => {
                    self.inst_pha();
                }
                Opcode::EorImm => {
                    let op = self.am_imm();
                    self.inst_eor(op);
                }
                Opcode::LsrAcc => {
                    self.a = self.inst_lsr(self.a);
                }
                Opcode::JmpAbs => {
                    let addr = self.am_abs();
                    self.inst_jmp(addr);
                }
                Opcode::EorAbs => {
                    let addr = self.am_abs();
                    let op = self.cpu_read(addr, false);
                    self.inst_eor(op);
                }
                Opcode::LsrAbs => {
                    let addr = self.am_abs();
                    let op = self.cpu_read(addr, false);
                    let result = self.inst_lsr(op);
                    self.cpu_write(addr, result);
                }

                Opcode::Bvc => {
                    let addr = self.am_rel();
                    self.inst_bvc(addr);
                }
                Opcode::EorIndY => {
                    let (addr, extra_cycle) = self.am_izy();
                    if extra_cycle {
                        self.cycles += 1;
                    }

                    let op = self.cpu_read(addr, false);
                    self.inst_eor(op);
                }
                Opcode::EorZpX => {
                    let addr = self.am_zpx();
                    let op = self.cpu_read(addr, false);
                    self.inst_eor(op);
                }
                Opcode::LsrZpX => {
                    let addr = self.am_zpx();
                    let op = self.cpu_read(addr, false);
                    let result = self.inst_lsr(op);
                    self.cpu_write(addr, result);
                }
                Opcode::Cli => {
                    self.inst_cli();
                }
                Opcode::EorAbsY => {
                    let (addr, extra_cycle) = self.am_aby();
                    if extra_cycle {
                        self.cycles += 1;
                    }

                    let op = self.cpu_read(addr, false);
                    self.inst_eor(op);
                }
                Opcode::EorAbsX => {
                    let (addr, extra_cycle) = self.am_abx();
                    if extra_cycle {
                        self.cycles += 1;
                    }

                    let op = self.cpu_read(addr, false);
                    self.inst_eor(op);
                }
                Opcode::LsrAbsX => {
                    let (addr, _) = self.am_abx();
                    let op = self.cpu_read(addr, false);
                    let result = self.inst_lsr(op);
                    self.cpu_write(addr, result);
                }

                Opcode::Rts => {
                    self.inst_rts();
                }
                Opcode::AdcIndX => {
                    let addr = self.am_izx();
                    let op = self.cpu_read(addr, false);
                    self.inst_adc(op);
                }
                Opcode::AdcZp => {
                    let addr = self.am_zp();
                    let op = self.cpu_read(addr, false);
                    self.inst_adc(op);
                }
                Opcode::RorZp => {
                    let addr = self.am_zp();
                    let op = self.cpu_read(addr, false);
                    let result = self.inst_ror(op);
                    self.cpu_write(addr, result);
                }
                Opcode::Pla => {
                    self.inst_pla();
                }
                Opcode::AdcImm => {
                    let op = self.am_imm();
                    self.inst_adc(op);
                }
                Opcode::RorAcc => {
                    self.a = self.inst_ror(self.a);
                }
                Opcode::JmpInd => {
                    let addr = self.am_ind();
                    self.inst_jmp(addr);
                }
                Opcode::AdcAbs => {
                    let addr = self.am_abs();
                    let op = self.cpu_read(addr, false);
                    self.inst_adc(op);
                }
                Opcode::RorAbs => {
                    let addr = self.am_abs();
                    let op = self.cpu_read(addr, false);
                    let result = self.inst_ror(op);
                    self.cpu_write(addr, result);
                }

                Opcode::Bvs => {
                    let addr = self.am_rel();
                    self.inst_bvs(addr);
                }
                Opcode::AdcIndY => {
                    let (addr, extra_cycle) = self.am_izy();
                    if extra_cycle {
                        self.cycles += 1;
                    }

                    let op = self.cpu_read(addr, false);
                    self.inst_adc(op);
                }
                Opcode::AdcZpX => {
                    let addr = self.am_zpx();
                    let op = self.cpu_read(addr, false);
                    self.inst_adc(op);
                }
                Opcode::RorZpX => {
                    let addr = self.am_zpx();
                    let op = self.cpu_read(addr, false);
                    let result = self.inst_ror(op);
                    self.cpu_write(addr, result);
                }
                Opcode::Sei => {
                    self.inst_sei();
                }
                Opcode::AdcAbsY => {
                    let (addr, extra_cycle) = self.am_aby();
                    if extra_cycle {
                        self.cycles += 1;
                    }

                    let op = self.cpu_read(addr, false);
                    self.inst_adc(op);
                }
                Opcode::AdcAbsX => {
                    let (addr, extra_cycle) = self.am_abx();
                    if extra_cycle {
                        self.cycles += 1;
                    }

                    let op = self.cpu_read(addr, false);
                    self.inst_adc(op);
                }
                Opcode::RorAbsX => {
                    let (addr, _) = self.am_abx();
                    let op = self.cpu_read(addr, false);
                    self.inst_ror(op);
                }

                Opcode::StaIndX => {
                    let addr = self.am_izx();
                    self.inst_sta(addr);
                }
                Opcode::StyZp => {
                    let addr = self.am_zp();
                    self.inst_sty(addr);
                }
                Opcode::StaZp => {
                    let addr = self.am_zp();
                    self.inst_sta(addr);
                }
                Opcode::StxZp => {
                    let addr = self.am_zp();
                    self.inst_stx(addr);
                }
                Opcode::Dey => {
                    self.inst_dey();
                }
                Opcode::Txa => {
                    self.inst_txa();
                }
                Opcode::StyAbs => {
                    let addr = self.am_abs();
                    self.inst_sty(addr);
                }
                Opcode::StaAbs => {
                    let addr = self.am_abs();
                    self.inst_sta(addr);
                }
                Opcode::StxAbs => {
                    let addr = self.am_abs();
                    self.inst_stx(addr);
                }

                Opcode::Bcc => {
                    let addr = self.am_rel();
                    self.inst_bcc(addr);
                }
                Opcode::StaIndY => {
                    let (addr, _) = self.am_izy();
                    self.inst_sta(addr);
                }
                Opcode::StyZpX => {
                    let addr = self.am_zpx();
                    self.inst_sty(addr);
                }
                Opcode::StaZpX => {
                    let addr = self.am_zpx();
                    self.inst_sta(addr);
                }
                Opcode::StxZpY => {
                    let addr = self.am_zpy();
                    self.inst_stx(addr);
                }
                Opcode::Tya => {
                    self.inst_tya();
                }
                Opcode::StaAbsY => {
                    let (addr, _) = self.am_aby();
                    self.inst_sta(addr);
                }
                Opcode::Txs => {
                    self.inst_txs();
                }
                Opcode::StaAbsX => {
                    let (addr, _) = self.am_abx();
                    self.inst_sta(addr);
                }

                Opcode::LdyImm => {
                    let op = self.am_imm();
                    self.inst_ldy(op);
                }
                Opcode::LdaIndX => {
                    let addr = self.am_izx();
                    let op = self.cpu_read(addr, false);
                    self.inst_lda(op);
                }
                Opcode::LdxImm => {
                    let op = self.am_imm();
                    self.inst_ldx(op);
                }
                Opcode::LdyZp => {
                    let addr = self.am_zp();
                    let op = self.cpu_read(addr, false);
                    self.inst_ldy(op);
                }
                Opcode::LdaZp => {
                    let addr = self.am_zp();
                    let op = self.cpu_read(addr, false);
                    self.inst_lda(op);
                }
                Opcode::LdxZp => {
                    let addr = self.am_zp();
                    let op = self.cpu_read(addr, false);
                    self.inst_ldx(op);
                }
                Opcode::Tay => {
                    self.inst_tay();
                }
                Opcode::LdaImm => {
                    let op = self.am_imm();
                    self.inst_lda(op);
                }
                Opcode::Tax => {
                    self.inst_tax();
                }
                Opcode::LdyAbs => {
                    let addr = self.am_abs();
                    let op = self.cpu_read(addr, false);
                    self.inst_ldy(op);
                }
                Opcode::LdaAbs => {
                    let addr = self.am_abs();
                    let op = self.cpu_read(addr, false);
                    self.inst_lda(op);
                }
                Opcode::LdxAbs => {
                    let addr = self.am_abs();
                    let op = self.cpu_read(addr, false);
                    self.inst_ldx(op);
                }

                Opcode::Bcs => {
                    let offset = self.am_rel();
                    self.inst_bcs(offset);
                }
                Opcode::LdaIndY => {
                    let (addr, extra_cycle) = self.am_izy();
                    if extra_cycle {
                        self.cycles += 1;
                    }

                    let op = self.cpu_read(addr, false);
                    self.inst_lda(op);
                }
                Opcode::LdyZpX => {
                    let addr = self.am_zpx();
                    let op = self.cpu_read(addr, false);
                    self.inst_ldy(op);
                }
                Opcode::LdaZpX => {
                    let addr = self.am_zpx();
                    let op = self.cpu_read(addr, false);
                    self.inst_lda(op);
                }
                Opcode::LdxZpY => {
                    let addr = self.am_zpy();
                    let op = self.cpu_read(addr, false);
                    self.inst_ldx(op);
                }
                Opcode::Clv => {
                    self.inst_clv();
                }
                Opcode::LdaAbsY => {
                    let (addr, extra_cycle) = self.am_aby();
                    if extra_cycle {
                        self.cycles += 1;
                    }

                    let op = self.cpu_read(addr, false);
                    self.inst_lda(op);
                }
                Opcode::Tsx => {
                    self.inst_tsx();
                }
                Opcode::LdyAbsX => {
                    let (addr, extra_cycle) = self.am_abx();
                    if extra_cycle {
                        self.cycles += 1;
                    }

                    let op = self.cpu_read(addr, false);
                    self.inst_ldy(op);
                }
                Opcode::LdaAbsX => {
                    let (addr, extra_cycle) = self.am_abx();
                    if extra_cycle {
                        self.cycles += 1;
                    }

                    let op = self.cpu_read(addr, false);
                    self.inst_lda(op);
                }
                Opcode::LdxAbsY => {
                    let (addr, extra_cycle) = self.am_aby();
                    if extra_cycle {
                        self.cycles += 1;
                    }

                    let op = self.cpu_read(addr, false);
                    self.inst_ldx(op);
                }

                Opcode::CpyImm => {
                    let op = self.am_imm();
                    self.inst_cpy(op);
                }
                Opcode::CmpIndX => {
                    let addr = self.am_izx();
                    let op = self.cpu_read(addr, false);
                    self.inst_cmp(op);
                }
                Opcode::CpyZp => {
                    let addr = self.am_zp();
                    let op = self.cpu_read(addr, false);
                    self.inst_cpy(op);
                }
                Opcode::CmpZp => {
                    let addr = self.am_izx();
                    let op = self.cpu_read(addr, false);
                    self.inst_cmp(op);
                }
                Opcode::DecZp => {
                    let addr = self.am_zp();
                    let op = self.cpu_read(addr, false);
                    let result = self.inst_dec(op);
                    self.cpu_write(addr, result);
                }
                Opcode::Iny => {
                    self.inst_iny();
                }
                Opcode::CmpImm => {
                    let op = self.am_imm();
                    self.inst_cmp(op);
                }
                Opcode::Dex => {
                    self.inst_dex();
                }
                Opcode::CpyAbs => {
                    let addr = self.am_abs();
                    let op = self.cpu_read(addr, false);
                    self.inst_cpy(op);
                }
                Opcode::CmpAbs => {
                    let addr = self.am_abs();
                    let op = self.cpu_read(addr, false);
                    self.inst_cmp(op);
                }
                Opcode::DecAbs => {
                    let addr = self.am_abs();
                    let op = self.cpu_read(addr, false);
                    let result = self.inst_dec(op);
                    self.cpu_write(addr, result);
                }

                Opcode::Bne => {
                    let offset = self.am_rel();
                    self.inst_bne(offset);
                }
                Opcode::CmpIndY => {
                    let (addr, extra_cycle) = self.am_izy();
                    if extra_cycle {
                        self.cycles += 1;
                    }

                    let op = self.cpu_read(addr, false);
                    self.inst_cmp(op);
                }
                Opcode::CmpZpX => {
                    let addr = self.am_zpx();
                    let op = self.cpu_read(addr, false);
                    self.inst_cmp(op);
                }
                Opcode::DecZpX => {
                    let addr = self.am_zpx();
                    let op = self.cpu_read(addr, false);
                    let result = self.inst_dec(op);
                    self.cpu_write(addr, result);
                }
                Opcode::Cld => {
                    self.inst_cld();
                }
                Opcode::CmpAbsY => {
                    let (addr, extra_cycle) = self.am_aby();
                    if extra_cycle {
                        self.cycles += 1;
                    }

                    let op = self.cpu_read(addr, false);
                    self.inst_cmp(op);
                }
                Opcode::CmpAbsX => {
                    let (addr, extra_cycle) = self.am_abx();
                    if extra_cycle {
                        self.cycles += 1;
                    }

                    let op = self.cpu_read(addr, false);
                    self.inst_cmp(op);
                }
                Opcode::DecAbsX => {
                    let (addr, _) = self.am_abx();
                    let op = self.cpu_read(addr, false);
                    let result = self.inst_dec(op);
                    self.cpu_write(addr, result);
                }

                Opcode::CpxImm => {
                    let op = self.am_imm();
                    self.inst_cpx(op);
                }
                Opcode::SbcIndX => {
                    let addr = self.am_izx();
                    let op = self.cpu_read(addr, false);
                    self.inst_sbc(op);
                }
                Opcode::CpxZp => {
                    let addr = self.am_zp();
                    let op = self.cpu_read(addr, false);
                    self.inst_cpx(op);
                }
                Opcode::SbcZp => {
                    let addr = self.am_zp();
                    let op = self.cpu_read(addr, false);
                    self.inst_sbc(op);
                }
                Opcode::IncZp => {
                    let addr = self.am_zp();
                    let op = self.cpu_read(addr, false);
                    let result = self.inst_inc(op);
                    self.cpu_write(addr, result);
                }
                Opcode::Inx => {
                    self.inst_inx();
                }
                Opcode::SbcImm => {
                    let op = self.am_imm();
                    self.inst_sbc(op);
                }
                Opcode::Nop => {
                    // This is intended, a NOP actually does nothing.
                }
                Opcode::CpxAbs => {
                    let addr = self.am_abs();
                    let op = self.cpu_read(addr, false);
                    self.inst_cpx(op);
                }
                Opcode::SbcAbs => {
                    let addr = self.am_abs();
                    let op = self.cpu_read(addr, false);
                    self.inst_sbc(op);
                }
                Opcode::IncAbs => {
                    let addr = self.am_abs();
                    let op = self.cpu_read(addr, false);
                    let result = self.inst_inc(op);
                    self.cpu_write(addr, result);
                }

                Opcode::Beq => {
                    let offset = self.am_rel();
                    self.inst_beq(offset);
                }
                Opcode::SbcIndY => {
                    let (addr, extra_cycle) = self.am_izy();
                    if extra_cycle {
                        self.cycles += 1;
                    }

                    let op = self.cpu_read(addr, false);
                    self.inst_sbc(op);
                }
                Opcode::SbcZpX => {
                    let addr = self.am_zpx();
                    let op = self.cpu_read(addr, false);
                    self.inst_sbc(op);
                }
                Opcode::IncZpX => {
                    let addr = self.am_zpx();
                    let op = self.cpu_read(addr, false);
                    let result = self.inst_inc(op);
                    self.cpu_write(addr, result);
                }
                Opcode::Sed => {
                    self.inst_sed();
                }
                Opcode::SbcAbsY => {
                    let (addr, extra_cycle) = self.am_aby();
                    if extra_cycle {
                        self.cycles += 1;
                    }

                    let op = self.cpu_read(addr, false);
                    self.inst_sbc(op);
                }
                Opcode::SbcAbsX => {
                    let (addr, extra_cycle) = self.am_abx();
                    if extra_cycle {
                        self.cycles += 1;
                    }

                    let op = self.cpu_read(addr, false);
                    self.inst_sbc(op);
                }
                Opcode::IncAbsX => {
                    let (addr, _) = self.am_abx();
                    let op = self.cpu_read(addr, false);
                    let result = self.inst_inc(op);
                    self.cpu_write(addr, result);
                }
            };

            self.cycles += opcode.cycles();
        }
        self.cycles -= 1;
    }

    // Addressing modes
    fn am_imm(&mut self) -> u8 {
        self.pc = self.pc.wrapping_add(1);
        self.cpu_read(self.pc.wrapping_sub(1), false)
    }

    fn am_zp(&mut self) -> u16 {
        self.pc = self.pc.wrapping_add(1);

        (self.cpu_read(self.pc.wrapping_sub(1), false) as u16) & 0x00ff
    }

    fn am_zpx(&mut self) -> u16 {
        self.pc = self.pc.wrapping_add(1);
        (self
            .cpu_read(self.pc.wrapping_sub(1), false)
            .wrapping_add(self.x) as u16)
            & 0x00ff
    }

    fn am_zpy(&mut self) -> u16 {
        self.pc = self.pc.wrapping_add(1);
        (self
            .cpu_read(self.pc.wrapping_sub(1), false)
            .wrapping_add(self.y) as u16)
            & 0x00ff
    }

    fn am_abs(&mut self) -> u16 {
        self.pc = self.pc.wrapping_add(2);
        (self.cpu_read(self.pc.wrapping_sub(2), false) as u16)
            | ((self.cpu_read(self.pc.wrapping_sub(1), false) as u16) << 8)
    }

    fn am_abx(&mut self) -> (u16, bool) {
        self.pc = self.pc.wrapping_add(2);
        let address_no_offset = (self.cpu_read(self.pc.wrapping_sub(2), false) as u16)
            | ((self.cpu_read(self.pc.wrapping_sub(1), false) as u16) << 8);
        let address_with_offset = address_no_offset.wrapping_add(self.x as u16);

        // Check if page has changed and request additionnal clock cycle
        let need_additionnal_cycle = address_no_offset & 0xff00 != address_with_offset & 0xff00;

        (address_with_offset, need_additionnal_cycle)
    }

    fn am_aby(&mut self) -> (u16, bool) {
        self.pc = self.pc.wrapping_add(2);
        let address_no_offset = (self.cpu_read(self.pc.wrapping_sub(2), false) as u16)
            | ((self.cpu_read(self.pc.wrapping_sub(1), false) as u16) << 8);
        let address_with_offset = address_no_offset.wrapping_add(self.y as u16);

        // Check if page has changed and request additionnal clock cycle
        let need_additionnal_cycle = address_no_offset & 0xff00 != address_with_offset & 0xff00;

        (address_with_offset, need_additionnal_cycle)
    }

    fn am_ind(&mut self) -> u16 {
        self.pc = self.pc.wrapping_add(2);

        let ptr = (self.cpu_read(self.pc.wrapping_sub(2), false) as u16)
            | ((self.cpu_read(self.pc.wrapping_sub(1), false) as u16) << 8);

        if ptr | 0x00ff == 0x00ff {
            // Simutate undefinied behavior at page end. The page is not updated.
            (self.cpu_read(ptr, false) as u16) | ((self.cpu_read(ptr & 0xff00, false) as u16) << 8)
        } else {
            (self.cpu_read(ptr, false) as u16)
                | ((self.cpu_read(ptr.wrapping_add(1), false) as u16) << 8)
        }
    }

    fn am_izx(&mut self) -> u16 {
        self.pc = self.pc.wrapping_add(1);
        let ptr = (self
            .cpu_read(self.pc.wrapping_sub(1), false)
            .wrapping_add(self.x) as u16)
            & 0x00ff;

        (self.cpu_read(ptr, false) as u16)
            | ((self.cpu_read(ptr.wrapping_add(1) & 0x00ff, false) as u16) << 8)
    }

    fn am_izy(&mut self) -> (u16, bool) {
        self.pc = self.pc.wrapping_add(1);
        let ptr = (self.cpu_read(self.pc.wrapping_sub(1), false) as u16) & 0x00ff;

        let address_no_offset = (self.cpu_read(ptr, false) as u16)
            | ((self.cpu_read(ptr.wrapping_add(1), false) as u16) << 8);

        let address_with_offset = address_no_offset.wrapping_add(self.y as u16);

        // Check if page has changed and request additionnal clock cycle
        let need_additionnal_cycle = address_no_offset & 0xff00 != address_with_offset & 0xff00;

        (address_with_offset, need_additionnal_cycle)
    }

    fn am_rel(&mut self) -> u16 {
        self.pc = self.pc.wrapping_add(1);

        let address = self.cpu_read(self.pc.wrapping_sub(1), false);

        // Sign expansion
        if address & 0x80 == 0x80 {
            (address as u16) | 0xff00
        } else {
            address as u16
        }
    }

    // Instructions
    fn inst_adc(&mut self, op: u8) {
        let mut result: u16 = (self.a as u16).wrapping_add(op as u16);

        if self.status_register.contains(StatusRegister::C) {
            result = result.wrapping_add(1);
        };

        let c = result > 0xff;
        self.status_register.set(StatusRegister::C, c);

        let r = (result & 0xff) as u8;

        let v = ((self.a ^ r) & !(self.a ^ op)) & 0x80 == 0x80;
        self.status_register.set(StatusRegister::C, v);

        self.a = r;

        let z = self.a == 0;
        self.status_register.set(StatusRegister::Z, z);

        let n = self.a & 0x80 == 0x80;
        self.status_register.set(StatusRegister::N, n);
    }

    fn inst_and(&mut self, op: u8) {
        self.a &= op;

        let z = self.a == 0;
        self.status_register.set(StatusRegister::Z, z);

        let n = self.a & 0x80 == 0x80;
        self.status_register.set(StatusRegister::N, n);
    }

    fn inst_asl(&mut self, op: u8) -> u8 {
        self.status_register
            .set(StatusRegister::C, op & 0x80 == 0x80);
        let result = op << 1;

        let z = self.a == 0;
        self.status_register.set(StatusRegister::Z, z);

        let n = self.a & 0x80 == 0x80;
        self.status_register.set(StatusRegister::N, n);

        result
    }

    fn inst_bcc(&mut self, offset: u16) {
        if !self.status_register.contains(StatusRegister::C) {
            self.branch(offset);
        }
    }

    fn inst_bcs(&mut self, offset: u16) {
        if self.status_register.contains(StatusRegister::C) {
            self.branch(offset);
        }
    }

    fn inst_beq(&mut self, offset: u16) {
        if self.status_register.contains(StatusRegister::Z) {
            self.branch(offset);
        }
    }

    fn inst_bit(&mut self, op: u8) {
        let result = self.a & op;

        self.status_register.set(StatusRegister::Z, result == 0);
        self.status_register
            .set(StatusRegister::V, result & (1 << 6) > 0);
        self.status_register
            .set(StatusRegister::N, result & (1 << 7) > 0);
    }

    fn inst_bmi(&mut self, offset: u16) {
        if self.status_register.contains(StatusRegister::N) {
            self.branch(offset);
        }
    }

    fn inst_bne(&mut self, offset: u16) {
        if !self.status_register.contains(StatusRegister::Z) {
            self.branch(offset);
        }
    }

    fn inst_bpl(&mut self, offset: u16) {
        if !self.status_register.contains(StatusRegister::N) {
            self.branch(offset);
        }
    }

    fn inst_brk(&mut self) {
        // Adds 1 to PC so we return to the next instruction
        self.pc = self.pc.wrapping_add(1);

        // Push current PC
        self.stack_push(((self.pc >> 8) & 0xff) as u8);
        self.stack_push((self.pc & 0xff) as u8);

        // Push status register
        self.status_register.set(StatusRegister::B, true);
        self.status_register.set(StatusRegister::U, true);
        self.stack_push(self.status_register.bits());

        self.status_register.set(StatusRegister::I, true);

        self.pc = self.cpu_read(IRQ_HANDLER, false) as u16
            | ((self.cpu_read(IRQ_HANDLER.wrapping_add(1), false) as u16) << 8);
    }

    fn inst_bvc(&mut self, offset: u16) {
        if !self.status_register.contains(StatusRegister::V) {
            self.branch(offset);
        }
    }

    fn inst_bvs(&mut self, offset: u16) {
        if self.status_register.contains(StatusRegister::V) {
            self.branch(offset);
        }
    }

    fn inst_clc(&mut self) {
        self.status_register.set(StatusRegister::C, false);
    }

    fn inst_cld(&mut self) {
        self.status_register.set(StatusRegister::D, false);
    }

    fn inst_cli(&mut self) {
        self.status_register.set(StatusRegister::I, false);
    }

    fn inst_clv(&mut self) {
        self.status_register.set(StatusRegister::V, false);
    }

    fn inst_cmp(&mut self, op: u8) {
        let result = self.a.wrapping_sub(op);

        let c = self.a >= op;
        self.status_register.set(StatusRegister::C, c);

        self.status_register.set(StatusRegister::Z, result == 0);

        self.status_register
            .set(StatusRegister::N, result & (1 << 7) > 0);
    }

    fn inst_cpx(&mut self, op: u8) {
        let result = self.x.wrapping_sub(op);

        let c = self.x >= op;
        self.status_register.set(StatusRegister::C, c);

        self.status_register.set(StatusRegister::Z, result == 0);

        self.status_register
            .set(StatusRegister::N, result & (1 << 7) > 0);
    }

    fn inst_cpy(&mut self, op: u8) {
        let result = self.y.wrapping_sub(op);

        let c = self.y >= op;
        self.status_register.set(StatusRegister::C, c);

        self.status_register.set(StatusRegister::Z, result == 0);

        self.status_register
            .set(StatusRegister::N, result & (1 << 7) > 0);
    }

    fn inst_dec(&mut self, op: u8) -> u8 {
        let result = op.wrapping_sub(1);

        self.status_register.set(StatusRegister::Z, result == 0);
        self.status_register
            .set(StatusRegister::N, result & (1 << 7) > 0);
        result
    }

    fn inst_dex(&mut self) {
        self.x = self.x.wrapping_sub(1);

        let z = self.x == 0;
        self.status_register.set(StatusRegister::Z, z);

        let n = self.x & (1 << 7) > 0;
        self.status_register.set(StatusRegister::N, n);
    }

    fn inst_dey(&mut self) {
        self.y = self.y.wrapping_sub(1);

        let z = self.y == 0;
        self.status_register.set(StatusRegister::Z, z);

        let n = self.y & (1 << 7) > 0;
        self.status_register.set(StatusRegister::N, n);
    }

    fn inst_eor(&mut self, op: u8) {
        self.a ^= op;

        let z = self.a == 0;
        self.status_register.set(StatusRegister::Z, z);

        let n = self.a & (1 << 7) > 0;
        self.status_register.set(StatusRegister::N, n);
    }

    fn inst_inc(&mut self, op: u8) -> u8 {
        let result = op.wrapping_add(1);

        self.status_register.set(StatusRegister::Z, result == 0);
        self.status_register
            .set(StatusRegister::N, result & (1 << 7) > 0);
        result
    }

    fn inst_inx(&mut self) {
        self.x = self.x.wrapping_add(1);

        let z = self.x == 0;
        self.status_register.set(StatusRegister::Z, z);

        let n = self.x & (1 << 7) > 0;
        self.status_register.set(StatusRegister::N, n);
    }

    fn inst_iny(&mut self) {
        self.y = self.y.wrapping_add(1);

        let z = self.y == 0;
        self.status_register.set(StatusRegister::Z, z);

        let n = self.y & (1 << 7) > 0;
        self.status_register.set(StatusRegister::N, n);
    }

    fn inst_jmp(&mut self, address: u16) {
        self.pc = address;
    }

    fn inst_jsr(&mut self, address: u16) {
        let pc = self.pc.wrapping_sub(1);

        self.stack_push((pc >> 8) as u8);
        self.stack_push((pc & 0x00ff) as u8);

        self.pc = address;
    }

    fn inst_lda(&mut self, op: u8) {
        self.a = op;

        let z = self.a == 0;
        self.status_register.set(StatusRegister::Z, z);

        let n = self.a & (1 << 7) > 0;
        self.status_register.set(StatusRegister::N, n);
    }

    fn inst_ldx(&mut self, op: u8) {
        self.x = op;

        let z = self.x == 0;
        self.status_register.set(StatusRegister::Z, z);

        let n = self.x & (1 << 7) > 0;
        self.status_register.set(StatusRegister::N, n);
    }

    fn inst_ldy(&mut self, op: u8) {
        self.y = op;

        let z = self.y == 0;
        self.status_register.set(StatusRegister::Z, z);

        let n = self.y & (1 << 7) > 0;
        self.status_register.set(StatusRegister::N, n);
    }

    fn inst_lsr(&mut self, op: u8) -> u8 {
        self.status_register
            .set(StatusRegister::C, op & (1 << 0) > 0);
        let result = op >> 1;

        self.status_register.set(StatusRegister::Z, result == 0);
        self.status_register
            .set(StatusRegister::N, result & (1 << 7) > 0);

        result
    }

    fn inst_ora(&mut self, op: u8) {
        self.a |= op;

        let z = self.a == 0;
        self.status_register.set(StatusRegister::Z, z);

        let n = self.a & (1 << 7) > 0;
        self.status_register.set(StatusRegister::N, n);
    }

    fn inst_pha(&mut self) {
        self.stack_push(self.a);
    }

    fn inst_php(&mut self) {
        self.status_register.set(StatusRegister::B, true);
        self.status_register.set(StatusRegister::U, true);

        self.stack_push(self.status_register.bits());

        self.status_register.set(StatusRegister::B, false);
        self.status_register.set(StatusRegister::U, false);
    }

    fn inst_pla(&mut self) {
        self.a = self.stack_pop();

        let z = self.a == 0;
        self.status_register.set(StatusRegister::Z, z);

        let n = self.a & 0x80 == 0x80;
        self.status_register.set(StatusRegister::N, n);
    }

    fn inst_plp(&mut self) {
        self.status_register = StatusRegister::from_bits_truncate(self.stack_pop());
        self.status_register.set(StatusRegister::B, false);
        self.status_register.set(StatusRegister::U, false);
    }

    fn inst_rol(&mut self, op: u8) -> u8 {
        let carry = self.status_register.contains(StatusRegister::C);

        self.status_register
            .set(StatusRegister::C, op & (1 << 7) > 0);

        let mut result = op << 1;

        if carry {
            result |= 1 << 0;
        }

        self.status_register.set(StatusRegister::Z, result == 0);
        self.status_register
            .set(StatusRegister::Z, result & (1 << 7) > 0);

        result
    }

    fn inst_ror(&mut self, op: u8) -> u8 {
        let carry = self.status_register.contains(StatusRegister::C);

        self.status_register
            .set(StatusRegister::C, op & (1 << 0) > 0);

        let mut result = op >> 1;

        if carry {
            result |= 1 << 7;
        }

        self.status_register.set(StatusRegister::Z, result == 0);
        self.status_register
            .set(StatusRegister::Z, result & (1 << 7) > 0);

        result
    }

    fn inst_rti(&mut self) {
        self.status_register = StatusRegister::from_bits_truncate(self.stack_pop());

        self.status_register.set(StatusRegister::B, false);
        self.status_register.set(StatusRegister::U, false);

        self.pc = self.stack_pop() as u16 | ((self.stack_pop() as u16) << 8);
    }

    fn inst_rts(&mut self) {
        self.pc = (self.stack_pop() as u16) | ((self.stack_pop() as u16) << 8);
        self.pc = self.pc.wrapping_add(1);
    }

    fn inst_sbc(&mut self, op: u8) {
        let op = op ^ 0xff;
        self.inst_adc(op);
    }

    fn inst_sec(&mut self) {
        self.status_register.set(StatusRegister::C, true);
    }

    fn inst_sed(&mut self) {
        self.status_register.set(StatusRegister::D, true);
    }

    fn inst_sei(&mut self) {
        self.status_register.set(StatusRegister::I, true);
    }

    fn inst_sta(&mut self, address: u16) {
        self.cpu_write(address, self.a);
    }

    fn inst_stx(&mut self, address: u16) {
        self.cpu_write(address, self.x);
    }

    fn inst_sty(&mut self, address: u16) {
        self.cpu_write(address, self.y);
    }

    fn inst_tax(&mut self) {
        self.x = self.a;

        let z = self.x == 0;
        self.status_register.set(StatusRegister::Z, z);

        let n = self.x & (1 << 7) > 0;
        self.status_register.set(StatusRegister::N, n);
    }

    fn inst_tay(&mut self) {
        self.y = self.a;

        let z = self.y == 0;
        self.status_register.set(StatusRegister::Z, z);

        let n = self.y & (1 << 7) > 0;
        self.status_register.set(StatusRegister::N, n);
    }

    fn inst_tsx(&mut self) {
        self.x = self.st;

        let z = self.x == 0;
        self.status_register.set(StatusRegister::Z, z);

        let n = self.x & (1 << 7) > 0;
        self.status_register.set(StatusRegister::N, n);
    }

    fn inst_txa(&mut self) {
        self.a = self.x;

        let z = self.a == 0;
        self.status_register.set(StatusRegister::Z, z);

        let n = self.a & (1 << 7) > 0;
        self.status_register.set(StatusRegister::N, n);
    }

    fn inst_txs(&mut self) {
        self.st = self.x;
    }

    fn inst_tya(&mut self) {
        self.a = self.y;

        let z = self.a == 0;
        self.status_register.set(StatusRegister::Z, z);

        let n = self.a & (1 << 7) > 0;
        self.status_register.set(StatusRegister::N, n);
    }

    // Other
    fn stack_push(&mut self, data: u8) {
        self.cpu_write(STACK_BASE.wrapping_add(self.st as u16), data);
        self.st = self.st.wrapping_sub(1);
    }

    fn stack_pop(&mut self) -> u8 {
        self.st = self.st.wrapping_add(1);
        self.cpu_read(STACK_BASE.wrapping_add(self.st as u16), false)
    }

    fn branch(&mut self, offset: u16) {
        self.cycles += 1;

        let result = self.pc.wrapping_add(offset);

        // If there is a page change, it takes an extra cycle
        if (result & 0xff00) != (self.pc & 0xff00) {
            self.cycles += 1;
        };

        self.pc = result;
    }
}
