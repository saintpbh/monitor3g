use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// Shared output state that controls which outputs are active.
/// This is shared between the UI thread and the compositor thread.
#[derive(Clone)]
pub struct OutputState {
    pub ndi_enabled: Arc<AtomicBool>,
    pub decklink_enabled: Arc<AtomicBool>,
}

impl OutputState {
    pub fn new() -> Self {
        Self {
            ndi_enabled: Arc::new(AtomicBool::new(false)),
            decklink_enabled: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn set_ndi(&self, enabled: bool) {
        self.ndi_enabled.store(enabled, Ordering::Relaxed);
        println!("[OutputState] NDI output: {}", if enabled { "ON" } else { "OFF" });
    }

    pub fn set_decklink(&self, enabled: bool) {
        self.decklink_enabled.store(enabled, Ordering::Relaxed);
        println!("[OutputState] DeckLink output: {}", if enabled { "ON" } else { "OFF" });
    }

    pub fn is_ndi_enabled(&self) -> bool {
        self.ndi_enabled.load(Ordering::Relaxed)
    }

    pub fn is_decklink_enabled(&self) -> bool {
        self.decklink_enabled.load(Ordering::Relaxed)
    }
}
