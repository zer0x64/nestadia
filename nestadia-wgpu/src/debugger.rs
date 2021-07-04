use crate::State;

use std::io::{stdin, stdout, Write};

use structopt::clap::AppSettings;
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(no_version)]
enum DebuggerOpt {
    #[structopt(alias = "c", no_version)]
    Continue,

    #[structopt(alias = "b", no_version)]
    Break {
        #[structopt(parse(try_from_str = parse_hex_addr))]
        addr: u16,
    },

    #[structopt(no_version)]
    Delete { index: Option<usize> },

    #[structopt(alias = "s", no_version)]
    Step,

    #[structopt(alias = "i", no_version)]
    Info(DebuggerInfoOpt),

    #[structopt(alias = "disas", no_version)]
    Disassemble {
        #[structopt(parse(try_from_str = parse_hex_addr))]
        search_addr: Option<u16>,
    },

    #[structopt(alias = "hex", no_version)]
    Hexdump {
        #[structopt(parse(try_from_str = parse_hex_addr))]
        start_addr: u16,
        #[structopt(parse(try_from_str = parse_hex_addr))]
        end_addr: u16,
    },
}

#[derive(Debug, StructOpt)]
#[structopt(no_version)]
enum DebuggerInfoOpt {
    #[structopt(alias = "b", no_version)]
    Break,
    #[structopt(alias = "r", no_version)]
    Reg { register: Option<String> },
}

fn parse_hex_addr(src: &str) -> Result<u16, std::num::ParseIntError> {
    let src = src.trim_start_matches("0x");
    u16::from_str_radix(src, 16)
}

impl State {
    pub fn debugger_prompt(&mut self) -> Option<[u8; 256 * 240]> {
        let mut frame = None;

        print!("debugger> ");
        stdout().flush().unwrap();

        let mut input = String::new();
        stdin().read_line(&mut input).unwrap();

        let tokens = input.split_ascii_whitespace();
        let clap = DebuggerOpt::clap()
            .global_setting(AppSettings::NoBinaryName)
            .global_setting(AppSettings::DisableVersion)
            .global_setting(AppSettings::VersionlessSubcommands)
            .get_matches_from_safe(tokens);

        match clap {
            Ok(clap) => {
                let opt = DebuggerOpt::from_clap(&clap);
                match opt {
                    DebuggerOpt::Continue => self.paused = false,
                    DebuggerOpt::Break { addr } => {
                        let disassembly = self.emulator.disassemble(0, 0);
                        let closest_addr = disassembly
                            .iter()
                            .min_by_key(|&(_, x, _)| (x.wrapping_sub(addr)))
                            .unwrap()
                            .1;

                        self.breakpoints.push(closest_addr);
                        println!("Added breakpoint at {:#06x}", closest_addr);
                    }
                    DebuggerOpt::Delete { index } => {
                        if let Some(index) = index {
                            let removed = self.breakpoints.remove(index);
                            println!("Removed breakpoint {}: {:#06x}", index, removed);
                        } else {
                            self.breakpoints.clear();
                            println!("Cleared all breakpoints");
                        }
                    }
                    DebuggerOpt::Step => {
                        let current_pc = self.emulator.cpu().pc;
                        while {
                            if let Some(step_frame) = self.emulator.clock() {
                                frame = Some(*step_frame);
                            }
                            self.emulator.cpu().cycles > 0 || self.emulator.cpu().pc == current_pc
                        } {}
                        println!("pc: {:#06x}", self.emulator.cpu().pc);
                    }
                    DebuggerOpt::Info(info) => match info {
                        DebuggerInfoOpt::Break => {
                            for (index, addr) in self.breakpoints.iter().enumerate() {
                                println!("Breakpoint {}: {:#06x}", index, addr);
                            }
                        }
                        DebuggerInfoOpt::Reg { register } => {
                            let cpu = self.emulator.cpu();
                            if let Some(register) = register {
                                match register.as_str() {
                                    "a" => println!("a: {:#06x}", cpu.a),
                                    "x" => println!("x: {:#06x}", cpu.x),
                                    "y" => println!("y: {:#06x}", cpu.y),
                                    "st" => println!("st: {:#06x}", cpu.st),
                                    "pc" => println!("pc: {:#06x}", cpu.pc),
                                    "status" => println!("status: {:#06x}", cpu.status_register),
                                    reg => println!("Unknown register: {}", reg),
                                }
                            } else {
                                println!(
                                    " a: {:#06x}      x: {:#06x}      y: {:#06x}",
                                    cpu.a, cpu.x, cpu.y
                                );
                                println!(
                                    "st: {:#06x}     pc: {:#06x} status: {:#06x}",
                                    cpu.st, cpu.pc, cpu.status_register
                                );
                            }
                        }
                    },
                    DebuggerOpt::Disassemble { search_addr } => {
                        let cpu = self.emulator.cpu();
                        let disassembly = self.emulator.disassemble(0, 0);

                        let center_addr = if let Some(search_addr) = search_addr {
                            search_addr
                        } else {
                            cpu.pc
                        };

                        for (prg_bank, addr, disas) in &disassembly {
                            if (*addr as usize) > (center_addr as usize) - 20
                                && (*addr as usize) < (center_addr as usize) + 20
                            {
                                let prefix = if (*addr as usize) == (cpu.pc as usize) {
                                    ">"
                                } else {
                                    " "
                                };

                                let bank = if let Some(prg_bank) = prg_bank {
                                    format!("{:#04x}:", prg_bank)
                                } else {
                                    String::from("    :")
                                };

                                println!("{} {}{:#06x}: {}", prefix, bank, addr, disas);
                            }
                        }
                    }
                    DebuggerOpt::Hexdump {
                        start_addr,
                        end_addr,
                    } => {
                        // Align start address to get 16 values
                        let start_addr = start_addr - (start_addr & 0xf);
                        let data = self.emulator.mem_dump(start_addr, end_addr);

                        for (i, chunk) in data.chunks(16).enumerate() {
                            let addr = start_addr + (16 * i as u16);

                            let bytes = chunk
                                .iter()
                                .map(|c| format!("{:02x}", c))
                                .collect::<Vec<_>>()
                                .join(" ");

                            let ascii = chunk
                                .iter()
                                .map(|num| {
                                    if *num >= 32 && *num <= 126 {
                                        (*num as char).to_string()
                                    } else {
                                        '.'.to_string()
                                    }
                                })
                                .collect::<Vec<_>>()
                                .join("");

                            println!("{:#06x}: {:47} {}", addr, bytes, ascii);
                        }
                    }
                }
            }
            Err(e) => println!("{}", e.message),
        }

        frame
    }
}
