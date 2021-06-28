/// State machine for the sprite evaluation phase.
#[derive(Clone, Copy)]
pub enum SpriteEvalutationState {
    /// Idle until the end of the evaluation when all 64 sprites have been evaluated
    Idle,

    /// Copy the Y value from one OAM to another and check if the sprite is in the scanline. Takes 2 cycles,
    CheckY,

    /// Copy the 3 values from one OAM to another. Takes 6 cycles, 2 per  values.
    /// The inner value represent the current index of the data to copy
    CopyOam(u8),

    /// All 8 sprites have been found. Look (in a broken way) if the sprite overflow flag needs to be set.
    /// Inner value is `m` register.
    EvaluateOverflow(u8),
}

impl Default for SpriteEvalutationState {
    fn default() -> Self {
        Self::Idle
    }
}

/// State of a sprite on the current scanline
#[derive(Clone, Copy)]
pub enum SpriteXCounter {
    /// The sprite is not on the scanline at all. Happens when < 8 sprites are on the scanline
    WontRender,

    /// The sprite has not been rendered yet. Inner value represents number of X remaining before starting rendering.
    NotRendered(u8),

    /// The sprite is being renderes. Inner value represents the number of pixels left.
    Rendering(u8),

    /// The sprite is fully rendered.
    Rendered,
}

impl Default for SpriteXCounter {
    fn default() -> Self {
        Self::WontRender
    }
}

/// State of the sprite 0 hit
#[derive(Clone, Copy)]
pub enum SpriteZeroHitState {
    /// The state machine is simply idle
    Idle,

    /// The sprite 0 has been loaded in secondary OAM
    IsInOam,

    /// The sprite 0 will be rendered on the current scanline
    /// Inner value represents if it is also in the OAM of the next
    OnCurrentScanline(bool),

    /// The sprite 0 hit triggers 2 cycle after the actual pixel is evaluated.
    /// The inner value represent the remaining cycles before the flag is set
    Delay(u8),
}

impl Default for SpriteZeroHitState {
    fn default() -> Self {
        Self::Idle
    }
}
