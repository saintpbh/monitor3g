/// Shared key state — atomic for lock-free access between UI and compositor threads.
///
/// Same pattern as OutputState: all state is stored in Arc<Atomic*> so both
/// the UI thread and compositor thread can read/write without locks.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

/// Pack RGB into a single u32: 0x00RRGGBB
fn pack_rgb(r: u8, g: u8, b: u8) -> u32 {
    ((r as u32) << 16) | ((g as u32) << 8) | (b as u32)
}

/// Unpack u32 → (R, G, B)
pub fn unpack_rgb(packed: u32) -> (u8, u8, u8) {
    (
        ((packed >> 16) & 0xFF) as u8,
        ((packed >> 8) & 0xFF) as u8,
        (packed & 0xFF) as u8,
    )
}

/// Shared hybrid key parameters
#[derive(Clone)]
pub struct KeyState {
    /// Master enable
    enabled: Arc<AtomicBool>,
    /// Key color as packed RGB (default: 0x00B140 = TV green)
    key_color: Arc<AtomicU32>,
    /// Color distance tolerance (0–200, default: 80)
    tolerance: Arc<AtomicU32>,
    /// Edge softness (0–100, default: 30)
    softness: Arc<AtomicU32>,
    /// Luma low threshold (0–255, default: 16)
    luma_low: Arc<AtomicU32>,
    /// Luma softness (0–100, default: 10)
    luma_softness: Arc<AtomicU32>,
    /// Spill suppression enabled
    spill_suppress: Arc<AtomicBool>,
    /// Temporal smoothing (0-100, default: 0) to reduce flicker
    smoothing: Arc<AtomicU32>,
}

impl KeyState {
    pub fn new() -> Self {
        Self {
            enabled: Arc::new(AtomicBool::new(false)),
            key_color: Arc::new(AtomicU32::new(pack_rgb(0x00, 0xB1, 0x40))), // TV green
            tolerance: Arc::new(AtomicU32::new(80)),
            softness: Arc::new(AtomicU32::new(30)),
            luma_low: Arc::new(AtomicU32::new(16)),
            luma_softness: Arc::new(AtomicU32::new(10)),
            spill_suppress: Arc::new(AtomicBool::new(true)),
            smoothing: Arc::new(AtomicU32::new(0)),
        }
    }

    // === Getters ===
    pub fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::Relaxed)
    }

    pub fn key_color(&self) -> (u8, u8, u8) {
        unpack_rgb(self.key_color.load(Ordering::Relaxed))
    }

    pub fn tolerance(&self) -> u32 {
        self.tolerance.load(Ordering::Relaxed)
    }

    pub fn softness(&self) -> u32 {
        self.softness.load(Ordering::Relaxed)
    }

    pub fn luma_low(&self) -> u32 {
        self.luma_low.load(Ordering::Relaxed)
    }

    pub fn luma_softness(&self) -> u32 {
        self.luma_softness.load(Ordering::Relaxed)
    }

    pub fn spill_suppress(&self) -> bool {
        self.spill_suppress.load(Ordering::Relaxed)
    }

    pub fn smoothing(&self) -> u32 {
        self.smoothing.load(Ordering::Relaxed)
    }

    // === Setters ===
    pub fn set_enabled(&self, enabled: bool) {
        self.enabled.store(enabled, Ordering::Relaxed);
        println!("[KeyState] Hybrid Key: {}", if enabled { "ON" } else { "OFF" });
    }

    pub fn set_key_color(&self, r: u8, g: u8, b: u8) {
        self.key_color.store(pack_rgb(r, g, b), Ordering::Relaxed);
        println!("[KeyState] Key color: #{:02X}{:02X}{:02X}", r, g, b);
    }

    pub fn set_tolerance(&self, val: u32) {
        self.tolerance.store(val.min(200), Ordering::Relaxed);
    }

    pub fn set_softness(&self, val: u32) {
        self.softness.store(val.min(100), Ordering::Relaxed);
    }

    pub fn set_luma_low(&self, val: u32) {
        self.luma_low.store(val.min(255), Ordering::Relaxed);
    }

    pub fn set_luma_softness(&self, val: u32) {
        self.luma_softness.store(val.min(100), Ordering::Relaxed);
    }

    pub fn set_spill_suppress(&self, enabled: bool) {
        self.spill_suppress.store(enabled, Ordering::Relaxed);
    }

    pub fn set_smoothing(&self, val: u32) {
        self.smoothing.store(val.min(100), Ordering::Relaxed);
    }
}
