#[cfg(target_os = "macos")]
pub mod mac;
#[cfg(target_os = "macos")]
#[allow(dead_code)]
pub mod mac_cg; // Legacy CoreGraphics capture (archived)
pub mod camera;

// Re-export for convenience
#[cfg(target_os = "macos")]
pub use mac::{list_displays, list_windows, list_sources, capture_display, capture_window, DisplayInfo, WindowInfo};

