// Core module: Compositor and main application logic
pub mod compositor;
pub mod output_state;

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::ffi::c_void;

/// Video frame with zero-copy data sharing via Arc
#[derive(Clone)]
pub struct VideoFrame {
    pub width: u32,
    pub height: u32,
    pub data: Arc<Vec<u8>>,
    pub timestamp: std::time::Instant,
}

/// Identifies which source is active
#[derive(Debug, Clone)]
pub enum SourceId {
    Camera(usize),
    Screen(u32),
    Window(u32),
    Ndi(String),
}

/// Shared stop signal for capture threads
#[derive(Clone)]
pub struct StopSignal(Arc<AtomicBool>);

impl StopSignal {
    pub fn new() -> Self { Self(Arc::new(AtomicBool::new(false))) }
    pub fn stop(&self) { self.0.store(true, Ordering::Relaxed); }
    pub fn is_stopped(&self) -> bool { self.0.load(Ordering::Relaxed) }
}

/// Resolution preset
#[derive(Debug, Clone, Copy)]
pub struct Resolution {
    pub width: u32,
    pub height: u32,
    pub label: &'static str,
}

pub const RESOLUTION_PRESETS: &[Resolution] = &[
    Resolution { width: 3840, height: 2160, label: "4K UHD" },
    Resolution { width: 2560, height: 1440, label: "QHD" },
    Resolution { width: 1920, height: 1080, label: "Full HD" },
    Resolution { width: 1280, height: 720,  label: "HD" },
    Resolution { width: 640,  height: 480,  label: "SD" },
    Resolution { width: 0,    height: 0,    label: "Native" },
];

/// Shared resolution state (atomic for lock-free access)
#[derive(Clone)]
pub struct ResolutionState {
    pub input_w: Arc<AtomicU32>,
    pub input_h: Arc<AtomicU32>,
    pub output_w: Arc<AtomicU32>,
    pub output_h: Arc<AtomicU32>,
}

impl ResolutionState {
    pub fn new() -> Self {
        Self {
            input_w: Arc::new(AtomicU32::new(1920)),
            input_h: Arc::new(AtomicU32::new(1080)),
            output_w: Arc::new(AtomicU32::new(1920)),
            output_h: Arc::new(AtomicU32::new(1080)),
        }
    }

    pub fn set_input(&self, w: u32, h: u32) {
        self.input_w.store(w, Ordering::Relaxed);
        self.input_h.store(h, Ordering::Relaxed);
        println!("[Resolution] Input: {}x{}", w, h);
    }

    pub fn set_output(&self, w: u32, h: u32) {
        self.output_w.store(w, Ordering::Relaxed);
        self.output_h.store(h, Ordering::Relaxed);
        println!("[Resolution] Output: {}x{}", w, h);
    }

    pub fn input(&self) -> (u32, u32) {
        (self.input_w.load(Ordering::Relaxed), self.input_h.load(Ordering::Relaxed))
    }

    pub fn output(&self) -> (u32, u32) {
        (self.output_w.load(Ordering::Relaxed), self.output_h.load(Ordering::Relaxed))
    }
}

// ========== macOS vImage hardware-accelerated scaling ==========

#[repr(C)]
struct vImage_Buffer {
    data: *mut c_void,
    height: usize,
    width: usize,
    row_bytes: usize,
}

#[link(name = "Accelerate", kind = "framework")]
unsafe extern "C" {
    fn vImageScale_ARGB8888(
        src: *const vImage_Buffer,
        dest: *const vImage_Buffer,
        temp_buffer: *mut c_void,
        flags: u32,
    ) -> i64;

    fn vImagePermuteChannels_ARGB8888(
        src: *const vImage_Buffer,
        dest: *const vImage_Buffer,
        permuteMap: *const u8,
        flags: u32,
    ) -> i64;
}

/// Hardware-accelerated BGRA → RGBA conversion via vImage (SIMD/NEON)
pub fn bgra_to_rgba_vimage(data: &mut Vec<u8>, width: u32, height: u32) {
    let buf = vImage_Buffer {
        data: data.as_mut_ptr() as *mut c_void,
        height: height as usize,
        width: width as usize,
        row_bytes: (width * 4) as usize,
    };

    // Permute map: BGRA[0,1,2,3] → RGBA = [2,1,0,3]
    let permute_map: [u8; 4] = [2, 1, 0, 3];

    let result = unsafe {
        vImagePermuteChannels_ARGB8888(&buf, &buf, permute_map.as_ptr(), 0)
    };

    if result != 0 {
        eprintln!("[vImage] Permute error: {}", result);
    }
}

/// Hardware-accelerated RGBA scaling via macOS vImage (Accelerate framework)
pub fn scale_rgba_vimage(
    src: &[u8], src_w: u32, src_h: u32,
    dst_w: u32, dst_h: u32,
) -> Vec<u8> {
    if dst_w == 0 || dst_h == 0 || (src_w == dst_w && src_h == dst_h) {
        return src.to_vec();
    }

    let mut dst = vec![0u8; (dst_w * dst_h * 4) as usize];

    let src_buf = vImage_Buffer {
        data: src.as_ptr() as *mut c_void,
        height: src_h as usize,
        width: src_w as usize,
        row_bytes: (src_w * 4) as usize,
    };

    let dst_buf = vImage_Buffer {
        data: dst.as_mut_ptr() as *mut c_void,
        height: dst_h as usize,
        width: dst_w as usize,
        row_bytes: (dst_w * 4) as usize,
    };

    // kvImageHighQualityResampling = (1 << 3) = 8, kvImageNoFlags = 0
    let result = unsafe {
        vImageScale_ARGB8888(&src_buf, &dst_buf, std::ptr::null_mut(), 0)
    };

    if result != 0 {
        eprintln!("[vImage] Scale error: {}", result);
    }

    dst
}
