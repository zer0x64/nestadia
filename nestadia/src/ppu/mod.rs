use crate::bus::PpuBus;

/// Registers definitions
pub mod registers;
pub mod sprites;
use sprites::{SpriteEvalutationState, SpriteXCounter, SpriteZeroHitState};

pub const FRAME_WIDTH: usize = 256;
pub const FRAME_HEIGHT: usize = 240;

pub type PpuFrame = [u8; FRAME_WIDTH * FRAME_HEIGHT];

pub struct Ppu {
    // Internal memory
    palette_table: [u8; 32],    // For color stuff
    oam_data: [u8; 64 * 4],     // Object Attribute Memory, internal to PPU
    secondary_oam: [u8; 8 * 4], // Object Attribute Memory of sprites to render on the scanline.

    // Rendering pipeline memory
    pattern_pipeline: [u16; 2], // Shift registers that contains the next 8 pixels
    palette_pipeline: [u16; 2], // Contains the palette attributes for the next 8 pixels
    sprites_pipeline: [u8; 8 * 2], // Contains the pattern info for the currently loaded sprites
    sprites_attributes: [u8; 8], // Attribute bytes for the currently loaded sprites
    sprites_x_counter: [SpriteXCounter; 8], // X counter for the currently loaded sprites
    sprite_evaluation_state: SpriteEvalutationState, // State machine for the sprite evaluation process
    oam_pointer: u8, // Pointer to a primary OAM entry during the sprite evaluation phase. Known as `n` on the wiki.
    secondary_oam_pointer: u8, // Pointer to the secondary OAM entry during the sprite evaluation phase.
    oam_latch: u8, // Buffer that contains the value between the read and the write between OAMs
    oam_temp_y_buffer: u8, // Temporary buffer to read a Y attribute before using it
    oam_temp_tile_buffer: u8, // Temporary buffer to read the tile info before using it

    // Registers
    ctrl_reg: registers::ControlReg,
    mask_reg: registers::MaskReg,
    status_reg: registers::StatusReg,
    oam_addr_reg: u8,
    vram_addr: registers::VramAddr,
    temp_vram_addr: registers::VramAddr,
    fine_x: u8,
    write_latch: bool,

    // Emulation-specific internal stuff
    cycle_count: u16,
    scanline: i16,
    frame: PpuFrame,
    vblank_nmi_set: bool,
    last_data_on_bus: u8,
    sprite_zero_hit_state: SpriteZeroHitState,
    is_odd_frame: bool,

    // Buffers for cycle-accurate reads
    nt_buffer: u8,
    at_buffer: u8,
    bg_lo_buffer: u8,
    bg_hi_buffer: u8,
}

impl Default for Ppu {
    fn default() -> Self {
        Self::new()
    }
}

impl Ppu {
    pub fn new() -> Self {
        Ppu {
            palette_table: [0u8; 32],
            oam_data: [0u8; 64 * 4],
            secondary_oam: [0xffu8; 8 * 4],

            pattern_pipeline: [0u16; 2],
            palette_pipeline: [0u16; 2],
            sprites_pipeline: [0u8; 8 * 2],
            sprites_attributes: [0u8; 8],
            sprites_x_counter: Default::default(),
            sprite_evaluation_state: Default::default(),
            oam_pointer: 0,
            secondary_oam_pointer: 0,
            oam_latch: 0,
            oam_temp_y_buffer: 0,
            oam_temp_tile_buffer: 0,

            ctrl_reg: registers::ControlReg::default(),
            mask_reg: registers::MaskReg::default(),
            status_reg: registers::StatusReg::default(),
            oam_addr_reg: 0,
            vram_addr: registers::VramAddr::default(),
            temp_vram_addr: registers::VramAddr::default(),
            fine_x: 0,
            write_latch: false,

            cycle_count: 0,
            scanline: -1,
            frame: [0u8; 256 * 240],
            vblank_nmi_set: false,
            last_data_on_bus: 0,
            sprite_zero_hit_state: Default::default(),
            is_odd_frame: false,

            nt_buffer: 0,
            at_buffer: 0,
            bg_lo_buffer: 0,
            bg_hi_buffer: 0,
        }
    }

    pub fn reset(&mut self) {
        *self = Default::default()
    }

    pub fn take_vblank_nmi_set_state(&mut self) -> bool {
        let state = self.vblank_nmi_set;
        self.vblank_nmi_set = false;
        state
    }

    pub fn write(&mut self, bus: &mut PpuBus<'_>, addr: u16, data: u8) {
        let addr = addr & 0x07; // mirror

        match addr {
            0 => {
                // Write Control register

                let prewrite_generate_nmi_ctrl_state =
                    self.ctrl_reg.contains(registers::ControlReg::GENERATE_NMI);

                self.ctrl_reg.write(data);

                self.temp_vram_addr.set_nametable((data & 0b11) as u16);

                let postwrite_generate_nmi_ctrl_state =
                    self.ctrl_reg.contains(registers::ControlReg::GENERATE_NMI);
                let is_in_vblank = self
                    .status_reg
                    .contains(registers::StatusReg::VBLANK_STARTED);

                if !prewrite_generate_nmi_ctrl_state
                    && postwrite_generate_nmi_ctrl_state
                    && is_in_vblank
                {
                    self.vblank_nmi_set = true;
                }
            }
            1 => {
                // Write Mask register
                self.mask_reg.write(data);
            }
            2 => {
                // Status - not writable
                log::warn!("Attempted to write read-only PPU address: {:#X}", addr);
            }
            3 => {
                // Write OAM Address
                self.oam_addr_reg = data;
            }
            4 => {
                // Write OAM Data
                self.oam_data[self.oam_addr_reg as usize] = data;
                // Writes increment OAM addr
                self.oam_addr_reg = self.oam_addr_reg.wrapping_add(1);
            }
            5 => {
                // Write scroll to t and fine_x
                if self.write_latch {
                    self.temp_vram_addr.set_coarse_y((data >> 3) as u16);
                    self.temp_vram_addr.set_fine_y((data & 0b111) as u16);
                } else {
                    self.temp_vram_addr.set_coarse_x((data >> 3) as u16);
                    self.fine_x = data & 0b111;
                };

                self.write_latch = !self.write_latch;
            }
            6 => {
                if self.write_latch {
                    let value = (self.temp_vram_addr.get() & 0xff00) | (data as u16);
                    self.temp_vram_addr.set(value);
                    self.vram_addr = self.temp_vram_addr;
                } else {
                    // For some reasons, bit 15 is cleared here
                    let value =
                        (self.temp_vram_addr.get() & 0x00ff) | (((data & 0x3f) as u16) << 8);
                    self.temp_vram_addr.set(value);
                };

                self.write_latch = !self.write_latch;
            }
            7 => {
                // Write PPU Data

                // Address to write data to
                let write_addr = self.vram_addr.get() & 0x3fff;

                // All PPU data writes increment the nametable addr
                self.increment_vram_addr();

                match write_addr {
                    // Addresses mapped to PPU bus
                    0..=0x1FFF => bus.write_chr_mem(write_addr, data),
                    0x2000..=0x2FFF => bus.write_name_tables(write_addr, data),

                    // Unused addresses
                    0x3000..=0x3EFF => log::warn!("address space 0x3000..0x3EFF is not expected to be used, but it was attempted to write at 0x{:#X}", write_addr),

                    // Palette table:
                    0x3F00..=0x3FFF => {
                        if write_addr & 0b11 == 0 {
                            // Mirror to the universal background color
                            self.palette_table[usize::from(write_addr & 0x0f)] = data;
                        } else {
                            self.palette_table[usize::from(write_addr & 0x1f)] = data;
                        }
                    },

                    _ => unreachable!("unexpected write to mirrored space {:#X}", write_addr),
                }
            }
            _ => {
                unreachable!("unexpected write to mirrored space {:#X}", addr);
            }
        }
    }

    pub fn write_oam_dma(&mut self, buffer: &[u8; 256]) {
        for data in buffer.iter() {
            self.oam_data[self.oam_addr_reg as usize] = *data;
            self.oam_addr_reg = self.oam_addr_reg.wrapping_add(1);
        }
    }

    pub fn read(&mut self, bus: &mut PpuBus<'_>, addr: u16) -> u8 {
        let addr = addr & 0x07; // mirror

        match addr {
            // Not readable addresses
            0 | 1 | 3 | 5 | 6 => {
                // Control, mask, OAM address, scroll, PPU Address
                log::warn!(
                    "Attempted to read write-only PPU address: {:#X} (culprit at {})",
                    addr,
                    core::panic::Location::caller()
                );
                0
            }

            // Readable addresses
            2 => {
                // Read Status

                // 3 top bits are the PPU status, least significant bits are noise from PPU bus.
                let snapshot = self.status_reg.read() | self.last_data_on_bus & 0x1F;

                // Reading the Status register clear bit 7 and also the address latch used by PPUSCROLL and PPUADDR.
                self.status_reg.remove(registers::StatusReg::VBLANK_STARTED);

                self.write_latch = false;

                snapshot
            }
            4 => {
                // Read OAM Data
                // Reads do not cause increment
                self.oam_data[self.oam_addr_reg as usize]
            }
            7 => {
                // Read PPU Data

                // Address to read data from
                let read_addr = self.vram_addr.get() & 0x3fff;

                // All PPU data reads increment the nametable addr
                self.increment_vram_addr();

                match read_addr {
                    // Addresses mapped to PPU bus
                    0..=0x1FFF => {
                        let data = self.last_data_on_bus;
                        self.last_data_on_bus = bus.read_chr_mem(read_addr);
                        data
                    }
                    0x2000..=0x2FFF => {
                        let data = self.last_data_on_bus;
                        self.last_data_on_bus = bus.read_name_tables(read_addr);
                        data
                    }

                    // Unused address space
                    0x3000..=0x3EFF => {
                        log::warn!("address space 0x3000..0x3EFF is not expected to be used, but 0x{:#X} was requested", read_addr);
                        0
                    }

                    // Palette table:
                    0x3F00..=0x3FFF => {
                        let color = if read_addr & 0b11 == 0 {
                            // Mirror to the universal background color
                            self.palette_table[usize::from(read_addr & 0x0f)]
                        } else {
                            self.palette_table[usize::from(read_addr & 0x1f)]
                        };

                        // Apply greyscale to reads
                        if self.mask_reg.contains(registers::MaskReg::GREYSCALE) {
                            color & 0x30
                        } else {
                            color
                        }
                    }

                    _ => unreachable!("unexpected access to mirrored space {:#X}", read_addr),
                }
            }

            _ => {
                unreachable!("unexpected access to mirrored space {:#X}", addr);
            }
        }
    }

    pub fn ready_frame(&mut self) -> Option<&PpuFrame> {
        if self.cycle_count == 256 && self.scanline == 239 {
            // Yeah! We got a frame ready
            Some(&self.frame)
        } else {
            None
        }
    }

    /// Returns frame when it's ready
    pub fn clock(&mut self, bus: &mut PpuBus) {
        self.cycle_count += 1;

        if self.cycle_count >= 341 {
            self.cycle_count = 0;
            self.scanline += 1;

            if self.scanline >= 261 {
                // http://wiki.nesdev.com/w/index.php/PPU_rendering#Pre-render_scanline_.28-1_or_261.29
                // scanline = -1 is the dummy scanline
                self.scanline = -1;

                // Skips a cycle on odd frame, only if rendering is enabled
                if self.is_odd_frame && self.rendering_enabled() {
                    self.cycle_count += 1
                };

                self.is_odd_frame = !self.is_odd_frame;
            }

            // Update the state machine to detect sprite 0
            match self.sprite_zero_hit_state {
                SpriteZeroHitState::IsInOam => {
                    self.sprite_zero_hit_state = SpriteZeroHitState::OnCurrentScanline(false)
                }
                SpriteZeroHitState::OnCurrentScanline(false) => {
                    self.sprite_zero_hit_state = SpriteZeroHitState::Idle
                }
                SpriteZeroHitState::OnCurrentScanline(true) => {
                    self.sprite_zero_hit_state = SpriteZeroHitState::OnCurrentScanline(false)
                }
                _ => {}
            }
        };

        // Handle sprite 0 delay
        if let SpriteZeroHitState::Delay(mut x) = self.sprite_zero_hit_state {
            x -= 1;

            if x == 0 {
                self.status_reg
                    .insert(registers::StatusReg::SPRITE_ZERO_HIT);
                self.sprite_zero_hit_state = SpriteZeroHitState::Idle
            } else {
                self.sprite_zero_hit_state = SpriteZeroHitState::Delay(x)
            }
        };

        // Pre-render scanline
        if self.scanline == -1 {
            if self.cycle_count == 1 {
                // Reset sprite 0 hit
                self.status_reg
                    .remove(registers::StatusReg::SPRITE_ZERO_HIT);

                // Reset sprite overflow flag
                self.status_reg
                    .remove(registers::StatusReg::SPRITE_OVERFLOW);

                // VBLANK is done
                self.status_reg.remove(registers::StatusReg::VBLANK_STARTED);
            } else if self.cycle_count >= 280 && self.cycle_count <= 304 && self.rendering_enabled()
            {
                self.vram_addr.reset_y(&self.temp_vram_addr);
            }
        };

        self.render_pixel();

        // This condition is there to ensure that the VRAM address does not get updated if rendering is turned off
        if self.rendering_enabled() {
            // Visible + pre-render scanline
            if self.scanline < 240 {
                if self.scanline == -1 {
                    // Sprites are not loaded during the pre-render scanline
                    self.sprites_x_counter = Default::default();
                } else {
                    self.sprites_load_cycle(bus);
                };

                // Load BG info of 2 next tiles
                if (self.cycle_count > 0 && self.cycle_count <= 256)
                    || (self.cycle_count >= 321 && self.cycle_count <= 336)
                {
                    self.pattern_pipeline[0] = self.pattern_pipeline[0].overflowing_shl(1).0;
                    self.pattern_pipeline[1] = self.pattern_pipeline[1].overflowing_shl(1).0;

                    self.palette_pipeline[0] = self.palette_pipeline[0].overflowing_shl(1).0;
                    self.palette_pipeline[1] = self.palette_pipeline[1].overflowing_shl(1).0;

                    self.bg_load_cycle(bus);
                } else if self.cycle_count == 257 {
                    self.vram_addr.reset_x(&self.temp_vram_addr);
                };
            }
        }

        if self.scanline == 241 && self.cycle_count == 1 {
            // This is the exact cycle the VBLANK starts
            self.status_reg.insert(registers::StatusReg::VBLANK_STARTED);
            if self.ctrl_reg.contains(registers::ControlReg::GENERATE_NMI) {
                self.vblank_nmi_set = true;
            }
        };
    }

    fn render_pixel(&mut self) {
        use core::convert::TryFrom;

        if self.scanline < 0
            || self.scanline > 239
            || self.cycle_count < 1
            || self.cycle_count > 256
        {
            return;
        };

        let x = self.cycle_count.wrapping_sub(1);
        let y = u16::try_from(self.scanline).unwrap();

        let (background_transparent, background_color) =
            if self.mask_reg.contains(registers::MaskReg::SHOW_BACKGROUND)
                && (x >= 8
                    || self
                        .mask_reg
                        .contains(registers::MaskReg::LEFTMOST_8PXL_BACKGROUND))
            {
                self.get_background_pixel()
            } else {
                // Transparent with default background color
                (true, self.palette_table[0])
            };

        let sprite_pixel = if self.mask_reg.contains(registers::MaskReg::SHOW_SPRITES) {
            // We still fetch the sprite pixel even if the leftmost 8 pixel are not rendered to make sure the X counters are updated.
            let sprite_pixel = self.get_sprite_pixel();

            if x >= 8
                || self
                    .mask_reg
                    .contains(registers::MaskReg::LEFTMOST_8PXL_SPRITE)
            {
                sprite_pixel
            } else {
                None
            }
        } else {
            None
        };

        if let Some((sprite_color, behind_background, is_sprite_zero)) = sprite_pixel {
            if background_transparent {
                // If background is transparent, render sprite
                self.set_pixel(x, y, sprite_color);
            } else {
                // Since both pixels are opaque, trigger sprite 0 hit if all other conditions are met
                if let SpriteZeroHitState::OnCurrentScanline(_) = self.sprite_zero_hit_state {
                    if is_sprite_zero && x != 255 {
                        self.sprite_zero_hit_state = SpriteZeroHitState::Delay(2)
                    }
                };

                if behind_background {
                    self.set_pixel(x, y, background_color);
                } else {
                    self.set_pixel(x, y, sprite_color);
                }
            }
        } else {
            // If there's not opaque sprite pixel, render background
            self.set_pixel(x, y, background_color);
        }
    }

    fn set_pixel(&mut self, x: u16, y: u16, color: u8) {
        let color = if self.mask_reg.contains(registers::MaskReg::GREYSCALE) {
            color & 0x30
        } else {
            color
        };

        let idx = y as usize * FRAME_WIDTH + x as usize;
        if idx < self.frame.len() {
            self.frame[idx] = color;
        }
    }

    fn increment_vram_addr(&mut self) {
        let inc_step = self.ctrl_reg.vram_addr_increment();

        self.vram_addr
            .set(self.vram_addr.get().wrapping_add(inc_step as u16) & 0x7fff)
    }

    fn get_background_pixel(&mut self) -> (bool, u8) {
        let fine_x = 15 - self.fine_x;
        let lo = ((self.pattern_pipeline[0] & (1 << fine_x)) >> fine_x) as u8;
        let hi = ((self.pattern_pipeline[1] & (1 << fine_x)) >> fine_x) as u8;

        let background_pat = hi << 1 | lo;

        let lo = ((self.palette_pipeline[0] & (1 << fine_x)) >> fine_x) as u8;
        let hi = ((self.palette_pipeline[1] & (1 << fine_x)) >> fine_x) as u8;

        let background_pal = hi << 1 | lo;

        let palette_index = if background_pat == 0 {
            0
        } else {
            (background_pal << 2) | background_pat
        };

        (
            background_pat == 0,
            self.palette_table[palette_index as usize],
        )
    }

    fn get_sprite_pixel(&mut self) -> Option<(u8, bool, bool)> {
        // Here I use this pattern to  make sure that all counter gets decremented even if the right pixel has been found
        let mut pixel = None;

        for sprite_idx in 0..self.sprites_x_counter.len() {
            match self.sprites_x_counter[sprite_idx] {
                SpriteXCounter::NotRendered(mut x) => {
                    x -= 1;
                    if x == 0 {
                        self.sprites_x_counter[sprite_idx] = SpriteXCounter::Rendering(0);
                    } else {
                        self.sprites_x_counter[sprite_idx] = SpriteXCounter::NotRendered(x);
                    }
                }
                SpriteXCounter::Rendering(mut fine_x) => {
                    fine_x += 1;
                    if fine_x == 8 {
                        // Sprite is done rendering
                        self.sprites_x_counter[sprite_idx] = SpriteXCounter::Rendered;
                    } else {
                        self.sprites_x_counter[sprite_idx] = SpriteXCounter::Rendering(fine_x);
                    };

                    // Only check colors if no pixel has been found
                    if pixel.is_none() {
                        let attributes = self.sprites_attributes[sprite_idx as usize];
                        let behind_background = attributes >> 5 & 1 == 1;
                        let palette_idx = attributes & 0b11;

                        let lo = self.sprites_pipeline[sprite_idx] & 0b1;
                        let hi = self.sprites_pipeline[8 | sprite_idx] & 0b1;

                        let sprite_pat = (hi << 1) | lo;

                        if sprite_pat != 0 {
                            let color = self.palette_table
                                [0x10 | ((palette_idx as usize) << 2) | (sprite_pat as usize)];

                            pixel = Some((color, behind_background, sprite_idx == 0));
                        }
                    }

                    self.sprites_pipeline[sprite_idx] >>= 1;
                    self.sprites_pipeline[8 | sprite_idx] >>= 1;
                }
                _ => {}
            };
        }

        pixel
    }

    fn bg_load_cycle(&mut self, bus: &mut PpuBus) {
        match (self.cycle_count - 1) & 0x7 {
            1 => {
                // Fetch NT byte
                let address = (self.vram_addr.get() & 0xfff) | 0x2000;
                self.nt_buffer = bus.read_name_tables(address);
            }
            3 => {
                // Fetch AT byte
                let address = 0x23c0
                    | ((self.vram_addr.nametable() as u16) << 10)
                    | (self.vram_addr.coarse_y() >> 2 << 3) as u16
                    | (self.vram_addr.coarse_x() >> 2) as u16;
                self.at_buffer = bus.read_name_tables(address);
            }
            5 => {
                // Compute lo BG tile byte
                let bank = self.ctrl_reg.background_pattern_base_addr();

                let fine_y = self.vram_addr.fine_y() as u16;
                let lo = bus.read_chr_mem(bank | (u16::from(self.nt_buffer) << 4) | fine_y);

                self.bg_lo_buffer = lo;
            }
            7 => {
                // Compute hi BG tile byte
                let bank = self.ctrl_reg.background_pattern_base_addr();

                let fine_y = self.vram_addr.fine_y() as u16;
                let hi =
                    bus.read_chr_mem(bank | (u16::from(self.nt_buffer) << 4) | (1 << 3) | fine_y);

                self.bg_hi_buffer = hi;

                // Feed pattern SR
                self.pattern_pipeline[0] |= self.bg_lo_buffer as u16;
                self.pattern_pipeline[1] |= self.bg_hi_buffer as u16;

                // Feed palette SR
                let quadrant_y = (self.vram_addr.coarse_y() >> 1) & 1;
                let quadrant_x = (self.vram_addr.coarse_x() >> 1) & 1;

                let bits = match (quadrant_x, quadrant_y) {
                    (0, 0) => self.at_buffer & 0b11,
                    (1, 0) => (self.at_buffer >> 2) & 0b11,
                    (0, 1) => (self.at_buffer >> 4) & 0b11,
                    (1, 1) => (self.at_buffer >> 6) & 0b11,
                    _ => unreachable!(),
                };

                let bits_lo = if bits & 0b1 == 1 { 0xff } else { 0x00 };

                let bits_hi = if bits & 0b10 == 0b10 { 0xff } else { 0x00 };

                self.palette_pipeline[0] |= bits_lo;
                self.palette_pipeline[1] |= bits_hi;

                // Increment vram address X and Y
                self.vram_addr.increment_coarse_x();

                if self.cycle_count == 256 {
                    self.vram_addr.increment_fine_y();
                };
            }
            _ => {}
        };
    }

    fn sprites_load_cycle(&mut self, bus: &mut PpuBus) {
        match self.cycle_count {
            0 => {
                // Initialization
                self.sprite_evaluation_state = SpriteEvalutationState::CheckY;
                self.oam_pointer = 0;
                self.secondary_oam_pointer = 0;
            }
            1..=64 => {
                // Only on even cycles
                if self.cycle_count & 1 == 0 {
                    self.secondary_oam[(self.cycle_count as usize - 1) >> 1] = 0xff;
                };
            }
            65..=256 => {
                match self.sprite_evaluation_state {
                    SpriteEvalutationState::Idle => { /* Idle*/ }
                    SpriteEvalutationState::CheckY => {
                        if self.cycle_count & 1 == 1 {
                            // On odd cycle, read value
                            self.oam_latch = self.oam_data[(self.oam_pointer << 2) as usize];
                        } else {
                            // On even cycle, write value
                            self.secondary_oam[(self.secondary_oam_pointer << 2) as usize] =
                                self.oam_latch;

                            // Check if y is in the scanline
                            let fine_y = (self.scanline as u8).wrapping_sub(self.oam_latch);

                            if fine_y < self.ctrl_reg.sprite_size() {
                                // Sprite is in scanline
                                self.sprite_evaluation_state = SpriteEvalutationState::CopyOam(1);

                                if self.oam_pointer == 0 {
                                    // This is sprite 0

                                    match self.sprite_zero_hit_state {
                                        SpriteZeroHitState::Idle => {
                                            self.sprite_zero_hit_state = SpriteZeroHitState::IsInOam
                                        }
                                        SpriteZeroHitState::OnCurrentScanline(_) => {
                                            self.sprite_zero_hit_state =
                                                SpriteZeroHitState::OnCurrentScanline(true)
                                        }
                                        _ => {}
                                    };
                                }
                            } else {
                                // Sprite is not in scanline
                                self.oam_pointer += 1;
                                if self.oam_pointer == 64 {
                                    // If all sprites are scanned, idle until the end of the evaluation
                                    self.sprite_evaluation_state = SpriteEvalutationState::Idle;
                                };
                            }
                        }
                    }
                    SpriteEvalutationState::CopyOam(m) => {
                        if self.cycle_count & 1 == 1 {
                            // Reads
                            self.oam_latch = self.oam_data[((self.oam_pointer << 2) | m) as usize];
                        } else {
                            // Writes
                            self.secondary_oam[((self.secondary_oam_pointer << 2) | m) as usize] =
                                self.oam_latch;

                            if m == 3 {
                                // Operation is done, increment pointers and switch state
                                self.oam_pointer += 1;
                                self.secondary_oam_pointer += 1;

                                if self.oam_pointer == 64 {
                                    // If all sprites are scanned, idle until the end of the evaluation
                                    self.sprite_evaluation_state = SpriteEvalutationState::Idle;
                                } else if self.secondary_oam_pointer < 8 {
                                    // Secondary OAM's not full, continue scanning
                                    self.sprite_evaluation_state = SpriteEvalutationState::CheckY;
                                } else {
                                    // Secondary OAM is full, check for sprite overflow
                                    self.sprite_evaluation_state =
                                        SpriteEvalutationState::EvaluateOverflow(0);
                                }
                            } else {
                                // Operation is not done, continue
                                self.sprite_evaluation_state =
                                    SpriteEvalutationState::CopyOam(m + 1);
                            }
                        };
                    }
                    SpriteEvalutationState::EvaluateOverflow(m) => {
                        // Buggy implementation to check if there is a sprite overflow
                        // This does NOT need to be perfect as barely any official games ever used it.
                        if self.cycle_count & 1 == 1 {
                            // On odd cycle, read value
                            self.oam_latch = self.oam_data[((self.oam_pointer << 2) | m) as usize];
                        } else {
                            // On even cycle, check for overflow
                            let fine_y = (self.scanline as u8).wrapping_sub(self.oam_latch);

                            if fine_y < self.ctrl_reg.sprite_size() {
                                // Overflow! Set the sprite overflow flag
                                self.status_reg
                                    .insert(registers::StatusReg::SPRITE_OVERFLOW);

                                // TODO: Technically not accurate, but I don't think there is any reason to
                                //  emulate the remaining reads(maybe for bus conflict emulation?), so we just Idle
                                self.sprite_evaluation_state = SpriteEvalutationState::Idle;
                            } else {
                                // Sprite does no hit, so n and m are (wrongly) incremented
                                self.oam_pointer += 1;

                                if self.oam_pointer == 64 {
                                    // All sprites have been evaluated, idle
                                    self.sprite_evaluation_state = SpriteEvalutationState::Idle;
                                } else {
                                    // There are still sprite to evaluate
                                    self.sprite_evaluation_state =
                                        SpriteEvalutationState::EvaluateOverflow(m + 1);
                                }
                            }
                        }
                    }
                }
            }
            257..=320 => {
                let sprite_idx = (self.cycle_count - 257) >> 3;
                let sprite_cycle = (self.cycle_count - 1) & 0b111;

                match sprite_cycle {
                    0 => {
                        self.oam_temp_y_buffer =
                            self.secondary_oam[((sprite_idx << 2) | sprite_cycle) as usize];
                    }
                    1 => {
                        self.oam_temp_tile_buffer =
                            self.secondary_oam[((sprite_idx << 2) | sprite_cycle) as usize];
                    }
                    2 => {
                        self.sprites_attributes[sprite_idx as usize] =
                            self.secondary_oam[((sprite_idx << 2) | sprite_cycle) as usize];
                    }
                    3 => {
                        let y = (self.scanline as u16).wrapping_sub(self.oam_temp_y_buffer as u16);

                        let x = self.secondary_oam[((sprite_idx << 2) | sprite_cycle) as usize];

                        self.sprites_x_counter[sprite_idx as usize] =
                            if y >= (self.ctrl_reg.sprite_size() as u16) {
                                SpriteXCounter::WontRender
                            } else if x == 0 {
                                SpriteXCounter::Rendering(0)
                            } else {
                                SpriteXCounter::NotRendered(x)
                            };
                    }
                    5 => {
                        let y = (self.scanline as u16).wrapping_sub(self.oam_temp_y_buffer as u16);
                        let attributes = self.sprites_attributes[sprite_idx as usize];

                        if self.ctrl_reg.sprite_size() == 8 {
                            // 8x8 sprites
                            let bank: u16 = self.ctrl_reg.sprite_pattern_base_addr();

                            let flipped_y = if attributes >> 7 & 1 == 1 {
                                // Y flipped
                                7u16.wrapping_sub(y)
                            } else {
                                y
                            };

                            let lo = bus.read_chr_mem(
                                bank | ((self.oam_temp_tile_buffer as u16) << 4)
                                    | (flipped_y as u16),
                            );

                            let lo = if attributes >> 6 & 1 == 1 {
                                // X flipped
                                lo
                            } else {
                                lo.reverse_bits()
                            };

                            self.sprites_pipeline[sprite_idx as usize] = lo;
                        } else {
                            // 8x16 sprites
                            let bank = if self.oam_temp_tile_buffer & 0b1 == 1 {
                                0x1000
                            } else {
                                0x0000
                            };
                            let tile_idx = self.oam_temp_tile_buffer as u16 & 0xfffe;

                            let flipped_y = if attributes >> 7 & 1 == 1 {
                                // It's flipped vertically
                                15u16.wrapping_sub(y)
                            } else {
                                y
                            };

                            // This is because of the hi/lo parts of the pattern memory
                            let flipped_y = if flipped_y >= 8 {
                                flipped_y.wrapping_add(8)
                            } else {
                                flipped_y
                            };

                            let lo = bus.read_chr_mem(bank | (tile_idx << 4) | (flipped_y as u16));

                            let lo = if attributes >> 6 & 1 == 1 {
                                // X flipped
                                lo
                            } else {
                                lo.reverse_bits()
                            };

                            self.sprites_pipeline[sprite_idx as usize] = lo;
                        }
                    }
                    7 => {
                        let y = (self.scanline as u16).wrapping_sub(self.oam_temp_y_buffer as u16);
                        let attributes = self.sprites_attributes[sprite_idx as usize];

                        if self.ctrl_reg.sprite_size() == 8 {
                            // 8x8 sprites
                            let bank: u16 = self.ctrl_reg.sprite_pattern_base_addr();

                            let flipped_y = if attributes >> 7 & 1 == 1 {
                                // Y flipped
                                7u16.wrapping_sub(y)
                            } else {
                                y
                            };

                            let hi = bus.read_chr_mem(
                                bank | 8
                                    | ((self.oam_temp_tile_buffer as u16) << 4)
                                    | (flipped_y as u16),
                            );

                            let hi = if attributes >> 6 & 1 == 1 {
                                // X flipped
                                hi
                            } else {
                                hi.reverse_bits()
                            };

                            self.sprites_pipeline[8 | sprite_idx as usize] = hi;
                        } else {
                            // 8x16 sprites
                            let bank = if self.oam_temp_tile_buffer & 0b1 == 1 {
                                0x1000
                            } else {
                                0x0000
                            };
                            let tile_idx = self.oam_temp_tile_buffer as u16 & 0xfffe;

                            let flipped_y = if attributes >> 7 & 1 == 1 {
                                // It's flipped vertically
                                15u16.wrapping_sub(y)
                            } else {
                                y
                            };

                            // This is because of the hi/lo parts of the pattern memory
                            let flipped_y = if flipped_y >= 8 {
                                flipped_y.wrapping_add(8)
                            } else {
                                flipped_y
                            };

                            let hi =
                                bus.read_chr_mem(bank | 8 | (tile_idx << 4) | (flipped_y as u16));

                            let hi = if attributes >> 6 & 1 == 1 {
                                // X flipped
                                hi
                            } else {
                                hi.reverse_bits()
                            };

                            self.sprites_pipeline[8 | sprite_idx as usize] = hi;
                        }
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }

    fn rendering_enabled(&self) -> bool {
        self.mask_reg.contains(registers::MaskReg::SHOW_BACKGROUND)
            || self.mask_reg.contains(registers::MaskReg::SHOW_SPRITES)
    }
}

#[cfg(test)]
pub mod test {
    use super::*;
    use crate::cartridge::Mirroring;
    use crate::Cartridge;

    const ROM_HORIZONTAL: &'static [u8] =
        include_bytes!("../../../default_roms/1.Branch_Basics.nes");
    const ROM_VERTICAL: &'static [u8] = include_bytes!("../../../default_roms/Alter_Ego.nes");

    struct MockEmulator {
        cartridge: Cartridge,
        ppu: Ppu,
        name_tables: [u8; 1024 * 4],
    }

    fn mock_emu(rom: &[u8]) -> MockEmulator {
        MockEmulator {
            cartridge: Cartridge::load(rom, None).unwrap(),
            ppu: Ppu::default(),
            name_tables: [0u8; 1024 * 4],
        }
    }

    fn mock_emu_horizontal() -> MockEmulator {
        mock_emu(ROM_HORIZONTAL)
    }

    fn mock_emu_vertical() -> MockEmulator {
        mock_emu(ROM_VERTICAL)
    }

    #[test]
    fn name_tables_writes() {
        let mut emu = mock_emu_horizontal();
        let mut bus = borrow_ppu_bus!(emu);

        emu.ppu.write(&mut bus, 0x2006, 0x23);
        emu.ppu.write(&mut bus, 0x2006, 0x05);
        emu.ppu.write(&mut bus, 0x2007, 0x66);

        assert_eq!(emu.name_tables[0x0305], 0x66);
    }

    #[test]
    fn name_tables_reads() {
        let mut emu = mock_emu_horizontal();
        emu.name_tables[0x0305] = 0x66;
        let mut bus = borrow_ppu_bus!(emu);

        emu.ppu.write(&mut bus, 0x2000, 0b0);

        emu.ppu.write(&mut bus, 0x2006, 0x23);
        emu.ppu.write(&mut bus, 0x2006, 0x05);

        assert_ne!(emu.ppu.read(&mut bus, 0x2007), 0x66); // dummy read, returns last data loaded on the bus
        assert_eq!(emu.ppu.vram_addr.get(), 0x2306); // address is incremented
        assert_eq!(emu.ppu.read(&mut bus, 0x2007), 0x66);
    }

    #[test]
    fn name_tables_reads_cross_page() {
        let mut emu = mock_emu_horizontal();
        emu.name_tables[0x01FF] = 0x66;
        emu.name_tables[0x0200] = 0x77;
        let mut bus = borrow_ppu_bus!(emu);

        emu.ppu.write(&mut bus, 0x2000, 0b0);

        emu.ppu.write(&mut bus, 0x2006, 0x21);
        emu.ppu.write(&mut bus, 0x2006, 0xFF);

        emu.ppu.read(&mut bus, 0x2007);
        assert_eq!(emu.ppu.read(&mut bus, 0x2007), 0x66);
        assert_eq!(emu.ppu.read(&mut bus, 0x2007), 0x77);
    }

    #[test]
    fn name_tables_reads_step_32() {
        let mut emu = mock_emu_horizontal();
        emu.name_tables[0x01FF] = 0x66;
        emu.name_tables[0x01FF + 32] = 0x77;
        emu.name_tables[0x01FF + 64] = 0x88;
        let mut bus = borrow_ppu_bus!(emu);

        emu.ppu.write(&mut bus, 0x2000, 0b100);

        emu.ppu.write(&mut bus, 0x2006, 0x21);
        emu.ppu.write(&mut bus, 0x2006, 0xFF);

        emu.ppu.read(&mut bus, 0x2007);
        assert_eq!(emu.ppu.read(&mut bus, 0x2007), 0x66);
        assert_eq!(emu.ppu.read(&mut bus, 0x2007), 0x77);
        assert_eq!(emu.ppu.read(&mut bus, 0x2007), 0x88);
    }

    // Horizontal
    // [0x2000 A ] [0x2400 a ]
    // [0x2800 B ] [0x2C00 b ]
    #[test]
    fn name_tables_horizontal_mirror() {
        let mut emu = mock_emu_horizontal();
        assert!(matches!(emu.cartridge.mirroring(), Mirroring::Horizontal));
        let mut bus = borrow_ppu_bus!(emu);

        // a
        emu.ppu.write(&mut bus, 0x2006, 0x24);
        emu.ppu.write(&mut bus, 0x2006, 0x05);
        emu.ppu.write(&mut bus, 0x2007, 0x66);

        // B
        emu.ppu.write(&mut bus, 0x2006, 0x28);
        emu.ppu.write(&mut bus, 0x2006, 0x05);
        emu.ppu.write(&mut bus, 0x2007, 0x77);

        // A
        emu.ppu.write(&mut bus, 0x2006, 0x20);
        emu.ppu.write(&mut bus, 0x2006, 0x05);
        emu.ppu.read(&mut bus, 0x2007);
        assert_eq!(emu.ppu.read(&mut bus, 0x2007), 0x66);

        // b
        emu.ppu.write(&mut bus, 0x2006, 0x2C);
        emu.ppu.write(&mut bus, 0x2006, 0x05);
        emu.ppu.read(&mut bus, 0x2007);
        assert_eq!(emu.ppu.read(&mut bus, 0x2007), 0x77);
    }

    // Vertical
    // [0x2000 A ] [0x2400 B ]
    // [0x2800 a ] [0x2C00 b ]
    #[test]
    fn name_tables_vertical_mirror() {
        let mut emu = mock_emu_vertical();
        assert!(matches!(emu.cartridge.mirroring(), Mirroring::Vertical));
        let mut bus = borrow_ppu_bus!(emu);

        // A
        emu.ppu.write(&mut bus, 0x2006, 0x20);
        emu.ppu.write(&mut bus, 0x2006, 0x05);
        emu.ppu.write(&mut bus, 0x2007, 0x66);

        // b
        emu.ppu.write(&mut bus, 0x2006, 0x2C);
        emu.ppu.write(&mut bus, 0x2006, 0x05);
        emu.ppu.write(&mut bus, 0x2007, 0x77);

        // a
        emu.ppu.write(&mut bus, 0x2006, 0x28);
        emu.ppu.write(&mut bus, 0x2006, 0x05);
        emu.ppu.read(&mut bus, 0x2007);
        assert_eq!(emu.ppu.read(&mut bus, 0x2007), 0x66);

        // B
        emu.ppu.write(&mut bus, 0x2006, 0x24);
        emu.ppu.write(&mut bus, 0x2006, 0x05);
        emu.ppu.read(&mut bus, 0x2007);
        assert_eq!(emu.ppu.read(&mut bus, 0x2007), 0x77);
    }

    #[test]
    fn name_tables_mirroring() {
        let mut emu = mock_emu_horizontal();
        emu.name_tables[0x0305] = 0x66;
        let mut bus = borrow_ppu_bus!(emu);

        emu.ppu.write(&mut bus, 0x2000, 0b0);

        emu.ppu.write(&mut bus, 0x2006, 0x63); // 0x6305 -> 0x2305
        emu.ppu.write(&mut bus, 0x2006, 0x05);

        emu.ppu.read(&mut bus, 0x2007);
        assert_eq!(emu.ppu.read(&mut bus, 0x2007), 0x66);
    }

    #[test]
    fn read_status_resets_latch() {
        let mut emu = mock_emu_vertical();
        emu.name_tables[0x0305] = 0x66;
        let mut bus = borrow_ppu_bus!(emu);

        emu.ppu.write(&mut bus, 0x2006, 0x21);
        emu.ppu.write(&mut bus, 0x2006, 0x23);
        emu.ppu.write(&mut bus, 0x2006, 0x05);

        emu.ppu.read(&mut bus, 0x2007);
        assert_ne!(emu.ppu.read(&mut bus, 0x2007), 0x66);

        emu.ppu.read(&mut bus, 0x2002);

        emu.ppu.write(&mut bus, 0x2006, 0x23);
        emu.ppu.write(&mut bus, 0x2006, 0x05);

        emu.ppu.read(&mut bus, 0x2007);
        assert_eq!(emu.ppu.read(&mut bus, 0x2007), 0x66);
    }

    #[test]
    fn read_status_resets_vblank() {
        let mut emu = mock_emu_horizontal();
        emu.ppu
            .status_reg
            .set(registers::StatusReg::VBLANK_STARTED, true);
        let mut bus = borrow_ppu_bus!(emu);

        assert_eq!(emu.ppu.read(&mut bus, 0x2002) >> 7, 1);
        assert_eq!(emu.ppu.status_reg.read() >> 7, 0);
    }

    #[test]
    fn oam_read_write() {
        let mut emu = mock_emu_horizontal();
        let mut bus = borrow_ppu_bus!(emu);

        emu.ppu.write(&mut bus, 0x2003, 0x10);
        emu.ppu.write(&mut bus, 0x2004, 0x66);
        emu.ppu.write(&mut bus, 0x2004, 0x77);

        emu.ppu.write(&mut bus, 0x2003, 0x10);
        assert_eq!(emu.ppu.read(&mut bus, 0x2004), 0x66);

        emu.ppu.write(&mut bus, 0x2003, 0x11);
        assert_eq!(emu.ppu.read(&mut bus, 0x2004), 0x77);
    }

    #[test]
    fn oam_dma() {
        let mut emu = mock_emu_horizontal();
        let mut bus = borrow_ppu_bus!(emu);

        let mut data = [0x66; 256];
        data[0] = 0x77;
        data[255] = 0x88;

        emu.ppu.write(&mut bus, 0x2003, 0x10);
        emu.ppu.write_oam_dma(&data);

        assert_eq!(emu.ppu.read(&mut bus, 0x2004), 0x77);
        emu.ppu.write(&mut bus, 0x2003, 0x0F); // "wrap around"
        assert_eq!(emu.ppu.read(&mut bus, 0x2004), 0x88);
    }
}
