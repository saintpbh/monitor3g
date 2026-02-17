#![allow(unsafe_op_in_unsafe_fn, non_snake_case)]
use core_graphics::display::{CGMainDisplayID, CGDisplayCreateImage};
use std::ffi::c_void;
use tokio::sync::mpsc;
use crate::core::{VideoFrame, StopSignal, ResolutionState, scale_rgba_vimage, bgra_to_rgba_vimage};
use std::sync::Arc;
use std::time::Instant;

pub struct DisplayInfo {
    pub id: u32,
    pub name: String,
    pub width: u32,
    pub height: u32,
}

pub struct WindowInfo {
    pub id: u32,
    pub owner: String,
    pub name: String,
    pub width: u32,
    pub height: u32,
}

pub fn list_displays() -> Vec<DisplayInfo> {
    let mut displays = Vec::new();
    unsafe {
        let mut display_ids = [0u32; 16];
        let mut count: u32 = 0;
        let result = CGGetActiveDisplayList(16, display_ids.as_mut_ptr(), &mut count);
        if result != 0 { return displays; }
        for i in 0..count as usize {
            let did = display_ids[i];
            let w = CGDisplayPixelsWide(did) as u32;
            let h = CGDisplayPixelsHigh(did) as u32;
            let is_main = did == CGMainDisplayID();
            let name = if is_main {
                format!("Main Display ({}x{})", w, h)
            } else {
                format!("Display {} ({}x{})", i + 1, w, h)
            };
            displays.push(DisplayInfo { id: did, name, width: w, height: h });
        }
    }
    println!("[Screen] Found {} display(s)", displays.len());
    displays
}

pub fn list_windows() -> Vec<WindowInfo> {
    let mut windows = Vec::new();
    unsafe {
        let options: u32 = (1 << 0) | (1 << 4);
        let window_list = CGWindowListCopyWindowInfo(options, 0);
        if window_list.is_null() { return windows; }

        let count = CFArrayGetCount(window_list);
        for i in 0..count {
            let dict = CFArrayGetValueAtIndex(window_list, i) as *const c_void;
            if dict.is_null() { continue; }

            let owner = get_string_from_dict(dict, "kCGWindowOwnerName");
            let name = get_string_from_dict(dict, "kCGWindowName");
            let wid = get_number_from_dict(dict, "kCGWindowNumber") as u32;
            let layer = get_number_from_dict(dict, "kCGWindowLayer") as i32;

            if layer != 0 || owner.is_empty() { continue; }

            let bounds_dict = get_dict_from_dict(dict, "kCGWindowBounds");
            let w = if !bounds_dict.is_null() { get_number_from_dict(bounds_dict, "Width") as u32 } else { 0 };
            let h = if !bounds_dict.is_null() { get_number_from_dict(bounds_dict, "Height") as u32 } else { 0 };

            if w < 100 || h < 100 { continue; }

            let display_name = if name.is_empty() { owner.clone() } else { format!("{} – {}", owner, name) };
            windows.push(WindowInfo { id: wid, owner, name: display_name, width: w, height: h });
        }
        CFRelease(window_list as *mut c_void);
    }
    println!("[Screen] Found {} window(s)", windows.len());
    windows
}

/// Capture display — NO sleep, channel backpressure throttles naturally
pub fn capture_display(display_id: u32, tx: mpsc::Sender<VideoFrame>, res: ResolutionState) -> StopSignal {
    let stop = StopSignal::new();
    let stop_clone = stop.clone();
    println!("[Screen] Starting display capture: ID {}", display_id);

    std::thread::spawn(move || {
        let mut frame_count: u64 = 0;
        while !stop_clone.is_stopped() {
            let image_ref = unsafe { CGDisplayCreateImage(display_id) };
            if !image_ref.is_null() {
                let src_w = unsafe { CGImageGetWidth(image_ref as *mut c_void) } as u32;
                let src_h = unsafe { CGImageGetHeight(image_ref as *mut c_void) } as u32;
                let raw = extract_pixel_data(image_ref as *mut c_void, src_w as usize, src_h as usize);
                unsafe { CGImageRelease(image_ref as *mut c_void); }

                // vImage hardware-accelerated scaling at capture time
                let (tw, th) = res.input();
                let (out_w, out_h, data) = if tw > 0 && th > 0 && (tw != src_w || th != src_h) {
                    let scaled = scale_rgba_vimage(&raw, src_w, src_h, tw, th);
                    (tw, th, scaled)
                } else {
                    (src_w, src_h, raw)
                };

                let frame = VideoFrame {
                    width: out_w, height: out_h,
                    data: Some(Arc::new(data)),
                    #[cfg(target_os = "macos")]
                    pixel_buffer: None,
                    timestamp: Instant::now(),
                };

                // blocking_send: channel buffer=2 naturally throttles capture rate
                if tx.blocking_send(frame).is_err() { break; }

                frame_count += 1;
                if frame_count % 60 == 0 {
                    println!("[Screen] {} frames ({}x{} → {}x{})", frame_count, src_w, src_h, out_w, out_h);
                }
            }
            // NO sleep — backpressure from channel buffer=2 regulates FPS
        }
        println!("[Screen] Display capture stopped.");
    });
    stop
}

/// Capture window — NO sleep
pub fn capture_window(window_id: u32, tx: mpsc::Sender<VideoFrame>, res: ResolutionState) -> StopSignal {
    let stop = StopSignal::new();
    let stop_clone = stop.clone();
    println!("[Screen] Starting window capture: ID {}", window_id);

    std::thread::spawn(move || {
        let mut frame_count: u64 = 0;
        while !stop_clone.is_stopped() {
            let image_ref = unsafe {
                CGWindowListCreateImage(
                    CGRect { x: 0.0, y: 0.0, w: 0.0, h: 0.0 },
                    1 << 3, window_id, 0,
                )
            };
            if !image_ref.is_null() {
                let src_w = unsafe { CGImageGetWidth(image_ref as *mut c_void) } as u32;
                let src_h = unsafe { CGImageGetHeight(image_ref as *mut c_void) } as u32;

                if src_w > 0 && src_h > 0 {
                    let raw = extract_pixel_data(image_ref as *mut c_void, src_w as usize, src_h as usize);
                    unsafe { CGImageRelease(image_ref as *mut c_void); }

                    let (tw, th) = res.input();
                    let (out_w, out_h, data) = if tw > 0 && th > 0 && (tw != src_w || th != src_h) {
                        let scaled = scale_rgba_vimage(&raw, src_w, src_h, tw, th);
                        (tw, th, scaled)
                    } else {
                        (src_w, src_h, raw)
                    };

                    let frame = VideoFrame {
                        width: out_w, height: out_h,
                        data: Some(Arc::new(data)),
                        #[cfg(target_os = "macos")]
                        pixel_buffer: None,
                        timestamp: Instant::now(),
                    };

                    if tx.blocking_send(frame).is_err() { break; }

                    frame_count += 1;
                    if frame_count % 60 == 0 {
                        println!("[Window] {} frames ({}x{} → {}x{})", frame_count, src_w, src_h, out_w, out_h);
                    }
                } else {
                    unsafe { CGImageRelease(image_ref as *mut c_void); }
                }
            }
            // NO sleep
        }
        println!("[Screen] Window capture stopped.");
    });
    stop
}

fn extract_pixel_data(image: *mut c_void, width: usize, height: usize) -> Vec<u8> {
    unsafe {
        let data_provider = CGImageGetDataProvider(image);
        if data_provider.is_null() { return vec![0u8; width * height * 4]; }
        let cf_data = CGDataProviderCopyData(data_provider);
        if cf_data.is_null() { return vec![0u8; width * height * 4]; }

        let length = CFDataGetLength(cf_data) as usize;
        let ptr = CFDataGetBytePtr(cf_data);
        let mut pixel_data = vec![0u8; length];
        std::ptr::copy_nonoverlapping(ptr, pixel_data.as_mut_ptr(), length);
        CFRelease(cf_data as *mut c_void);

        // BGRA → RGBA via vImage SIMD/NEON (replaces 15ms CPU loop)
        bgra_to_rgba_vimage(&mut pixel_data, width as u32, height as u32);
        pixel_data
    }
}

unsafe fn get_string_from_dict(dict: *const c_void, key: &str) -> String {
    let cf_key = cfstring_from_str(key);
    let value = CFDictionaryGetValue(dict, cf_key as *const c_void);
    CFRelease(cf_key as *mut c_void);
    if value.is_null() { return String::new(); }
    let mut buf = [0u8; 512];
    let ok = CFStringGetCString(value as *const c_void, buf.as_mut_ptr(), 512, 0x08000100);
    if ok {
        std::ffi::CStr::from_ptr(buf.as_ptr() as *const i8).to_string_lossy().into_owned()
    } else { String::new() }
}

unsafe fn get_number_from_dict(dict: *const c_void, key: &str) -> i64 {
    let cf_key = cfstring_from_str(key);
    let value = CFDictionaryGetValue(dict, cf_key as *const c_void);
    CFRelease(cf_key as *mut c_void);
    if value.is_null() { return 0; }
    let mut num: i64 = 0;
    CFNumberGetValue(value as *const c_void, 4, &mut num as *mut i64 as *mut c_void);
    num
}

unsafe fn get_dict_from_dict(dict: *const c_void, key: &str) -> *const c_void {
    let cf_key = cfstring_from_str(key);
    let value = CFDictionaryGetValue(dict, cf_key as *const c_void);
    CFRelease(cf_key as *mut c_void);
    value
}

unsafe fn cfstring_from_str(s: &str) -> *mut c_void {
    let cstr = std::ffi::CString::new(s).unwrap();
    CFStringCreateWithCString(std::ptr::null(), cstr.as_ptr(), 0x08000100)
}

#[repr(C)]
struct CGRect { x: f64, y: f64, w: f64, h: f64 }

#[link(name = "CoreGraphics", kind = "framework")]
unsafe extern "C" {
    fn CGImageGetWidth(image: *mut c_void) -> usize;
    fn CGImageGetHeight(image: *mut c_void) -> usize;
    fn CGImageRelease(image: *mut c_void);
    fn CGImageGetDataProvider(image: *mut c_void) -> *mut c_void;
    fn CGDataProviderCopyData(provider: *mut c_void) -> *mut c_void;
    fn CGGetActiveDisplayList(max: u32, displays: *mut u32, count: *mut u32) -> i32;
    fn CGDisplayPixelsWide(display: u32) -> usize;
    fn CGDisplayPixelsHigh(display: u32) -> usize;
    fn CGWindowListCopyWindowInfo(option: u32, relative_to: u32) -> *mut c_void;
    fn CGWindowListCreateImage(rect: CGRect, list_option: u32, window_id: u32, image_option: u32) -> *mut c_void;
}

#[link(name = "CoreFoundation", kind = "framework")]
unsafe extern "C" {
    fn CFDataGetLength(data: *mut c_void) -> isize;
    fn CFDataGetBytePtr(data: *mut c_void) -> *const u8;
    fn CFRelease(cf: *mut c_void);
    fn CFArrayGetCount(array: *mut c_void) -> isize;
    fn CFArrayGetValueAtIndex(array: *mut c_void, idx: isize) -> *const c_void;
    fn CFDictionaryGetValue(dict: *const c_void, key: *const c_void) -> *const c_void;
    fn CFStringCreateWithCString(alloc: *const c_void, cstr: *const i8, encoding: u32) -> *mut c_void;
    fn CFStringGetCString(str: *const c_void, buf: *mut u8, buf_size: isize, encoding: u32) -> bool;
    fn CFNumberGetValue(number: *const c_void, type_: i32, value_ptr: *mut c_void) -> bool;
}
