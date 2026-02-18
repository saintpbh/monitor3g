// Core module: Compositor and main application logic
pub mod compositor;
pub mod output_state;
pub mod key_state;
pub mod hybrid_key;
pub mod blender;

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
#[cfg(target_os = "macos")]
use screencapturekit::cv::{CVPixelBuffer, CVPixelBufferLockFlags};
use std::ffi::c_void;

/// Video frame with zero-copy data sharing via Arc
#[derive(Clone)]
pub struct VideoFrame {
    pub width: u32,
    pub height: u32,
    pub data: Option<Arc<Vec<u8>>>,
    #[cfg(target_os = "macos")]
    pub pixel_buffer: Option<std::sync::Arc<screencapturekit::cv::CVPixelBuffer>>,
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

/// Pack width+height into a single u64 for atomic read/write (prevents torn reads)
fn pack_resolution(w: u32, h: u32) -> u64 {
    ((w as u64) << 32) | (h as u64)
}
fn unpack_resolution(packed: u64) -> (u32, u32) {
    ((packed >> 32) as u32, packed as u32)
}

/// Shared resolution state (atomic for lock-free access)
/// Uses AtomicU64 packing to prevent torn reads (width/height always consistent)
#[derive(Clone)]
pub struct ResolutionState {
    input: Arc<AtomicU64>,
    output: Arc<AtomicU64>,
}

impl ResolutionState {
    pub fn new() -> Self {
        Self {
            input: Arc::new(AtomicU64::new(pack_resolution(1920, 1080))),
            output: Arc::new(AtomicU64::new(pack_resolution(1920, 1080))),
        }
    }

    pub fn set_input(&self, w: u32, h: u32) {
        self.input.store(pack_resolution(w, h), Ordering::Release);
        println!("[Resolution] Input: {}x{}", w, h);
    }

    pub fn set_output(&self, w: u32, h: u32) {
        self.output.store(pack_resolution(w, h), Ordering::Release);
        println!("[Resolution] Output: {}x{}", w, h);
    }

    pub fn input(&self) -> (u32, u32) {
        unpack_resolution(self.input.load(Ordering::Acquire))
    }

    pub fn output(&self) -> (u32, u32) {
        unpack_resolution(self.output.load(Ordering::Acquire))
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

#[cfg(target_os = "macos")]
pub fn extract_bgra_from_pixel_buffer(pixel_buffer: &CVPixelBuffer) -> Option<(u32, u32, Vec<u8>)> {
    let width = pixel_buffer.width() as u32;
    let height = pixel_buffer.height() as u32;
    if width == 0 || height == 0 { return None; }

    // RAII lock — automatically unlocks when guard drops
    let guard = pixel_buffer.lock(CVPixelBufferLockFlags::READ_ONLY).ok()?;
    let bytes_per_row = guard.bytes_per_row();
    let src_data = guard.as_slice();

    if src_data.is_empty() { return None; }

    let dst_stride = (width * 4) as usize;

    // If bytes_per_row matches, single memcpy
    if bytes_per_row == dst_stride {
        let expected = dst_stride * height as usize;
        if src_data.len() >= expected {
            return Some((width, height, src_data[..expected].to_vec()));
        }
    }

    // Row-by-row copy to remove padding
    let mut data = vec![0u8; dst_stride * height as usize];
    for y in 0..height as usize {
        let src_offset = y * bytes_per_row;
        let dst_offset = y * dst_stride;
        let copy_len = dst_stride.min(bytes_per_row);
        if src_offset + copy_len <= src_data.len() {
            if let Some(dst_slice) = data.get_mut(dst_offset..dst_offset + copy_len) {
                 dst_slice.copy_from_slice(&src_data[src_offset..src_offset + copy_len]);
            }
        }
    }
    
    if data.is_empty() { return None; }
    Some((width, height, data))
}

/// Efficiently scale and convert CVPixelBuffer directly to RGBA Vec<u8>
/// This avoids a redundant full-resolution copy.
#[cfg(target_os = "macos")]
pub fn scale_pixel_buffer_vimage(
    pixel_buffer: &CVPixelBuffer,
    dst_w: u32, dst_h: u32,
    convert_to_rgba: bool,
) -> Option<Vec<u8>> {
    let src_w = pixel_buffer.width() as u32;
    let src_h = pixel_buffer.height() as u32;
    if src_w == 0 || src_h == 0 || dst_w == 0 || dst_h == 0 { return None; }

    // RAII lock
    let guard = pixel_buffer.lock(CVPixelBufferLockFlags::READ_ONLY).ok()?;
    let src_data = guard.as_slice();
    let src_row_bytes = guard.bytes_per_row();

    let mut dst = vec![0u8; (dst_w * dst_h * 4) as usize];
    let dst_row_bytes = (dst_w * 4) as usize;

    let src_buf = vImage_Buffer {
        data: src_data.as_ptr() as *mut c_void,
        height: src_h as usize,
        width: src_w as usize,
        row_bytes: src_row_bytes,
    };

    let dst_buf = vImage_Buffer {
        data: dst.as_mut_ptr() as *mut c_void,
        height: dst_h as usize,
        width: dst_w as usize,
        row_bytes: dst_row_bytes,
    };

    // 1. Scale
    let result = unsafe {
        vImageScale_ARGB8888(&src_buf, &dst_buf, std::ptr::null_mut(), 0)
    };

    if result != 0 {
        eprintln!("[vImage] Scale error from pixel buffer: {}", result);
        return None;
    }

    // 2. Convert BGRA -> RGBA if requested
    if convert_to_rgba {
        let permute_map: [u8; 4] = [2, 1, 0, 3];
        let res = unsafe {
            vImagePermuteChannels_ARGB8888(&dst_buf, &dst_buf, permute_map.as_ptr(), 0)
        };
        if res != 0 {
            eprintln!("[vImage] Permute error: {}", res);
        }
    }

    Some(dst)
}
