/// Virtual Display — Rust FFI wrapper for CGVirtualDisplay
/// 
/// Creates a macOS virtual monitor that PowerPoint/Keynote can use
/// as a slide show output target. LuminaTV captures this display
/// via ScreenCaptureKit zero-copy pipeline.
/// 
/// Usage:
///   let vd = VirtualDisplay::new(1920, 1080, "LuminaTV Virtual")?;
///   let display_id = vd.display_id();
///   // Use capture_display(display_id, tx, res) to capture
///   // Drop `vd` to destroy the virtual display

use std::ffi::CString;
use std::os::raw::c_void;

unsafe extern "C" {
    fn lumina_create_virtual_display(width: u32, height: u32, name: *const i8) -> *mut c_void;
    fn lumina_get_virtual_display_id(display: *mut c_void) -> u32;
    fn lumina_destroy_virtual_display(display: *mut c_void);
}

/// Safe wrapper around the CGVirtualDisplay Obj-C object.
/// Automatically destroys the virtual display on Drop.
pub struct VirtualDisplay {
    ptr: *mut c_void,
    display_id: u32,
}

// Safety: The Obj-C object is thread-safe (ARC managed, no mutable state after creation)
unsafe impl Send for VirtualDisplay {}
unsafe impl Sync for VirtualDisplay {}

impl VirtualDisplay {
    /// Create a new virtual display with the given resolution and name.
    /// Returns None if CGVirtualDisplay API is not available (macOS < 14).
    pub fn new(width: u32, height: u32, name: &str) -> Option<Self> {
        let c_name = CString::new(name).ok()?;
        
        let ptr = unsafe { lumina_create_virtual_display(width, height, c_name.as_ptr()) };
        
        if ptr.is_null() {
            eprintln!("[VirtualDisplay] Failed to create virtual display (macOS 14+ required)");
            return None;
        }
        
        let display_id = unsafe { lumina_get_virtual_display_id(ptr) };
        
        if display_id == 0 {
            eprintln!("[VirtualDisplay] Virtual display created but has invalid ID");
            unsafe { lumina_destroy_virtual_display(ptr); }
            return None;
        }
        
        println!("[VirtualDisplay] Ready: '{}' {}x{} → Display ID {}", name, width, height, display_id);
        
        Some(VirtualDisplay { ptr, display_id })
    }
    
    /// Get the CGDirectDisplayID for this virtual display.
    /// Pass this to `capture_display()` for zero-copy capture.
    pub fn display_id(&self) -> u32 {
        self.display_id
    }
}

impl Drop for VirtualDisplay {
    fn drop(&mut self) {
        if !self.ptr.is_null() {
            println!("[VirtualDisplay] Shutting down virtual display (ID {})", self.display_id);
            unsafe { lumina_destroy_virtual_display(self.ptr); }
            self.ptr = std::ptr::null_mut();
        }
    }
}
