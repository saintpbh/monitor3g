/// ScreenCaptureKit-based screen capture (macOS 12.3+)
/// Zero-copy GPU pipeline: SCStream → IOSurface → CVPixelBuffer → BGRA data
///
/// Key advantage: ScreenCaptureKit does hardware GPU scaling at the OS level.
/// We request the target resolution via SCStreamConfiguration, and the OS
/// delivers frames already scaled — no vImage or CPU scaling needed.

use screencapturekit::prelude::*;
use screencapturekit::cv::{CVPixelBuffer, CVPixelBufferLockFlags};
use tokio::sync::mpsc;
use crate::core::{VideoFrame, StopSignal, ResolutionState};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::Instant;

// Re-export types for main.rs compatibility
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

// ========== Source Enumeration ==========

// ========== Source Enumeration ==========

pub fn list_sources() -> (Vec<DisplayInfo>, Vec<WindowInfo>) {
    let content = match SCShareableContent::get() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("[SCK] Failed to get shareable content: {:?}", e);
            
            // TCC Permission Denied Handling
            // If we can't get content, it's almost certainly a permission issue.
            // Prompt the user and open System Settings.
            use std::process::Command;
            
            let _ = Command::new("osascript")
                .arg("-e")
                .arg("display dialog \"LuminaTV requires Screen Recording permissions to capture your desktop.\n\nPlease enable it in System Settings > Privacy & Security > Screen Recording.\" with title \"Permission Required\" buttons {\"Open Settings\", \"Cancel\"} default button \"Open Settings\" cancel button \"Cancel\"")
                .output(); // execution blocks until user clicks

            // If user didn't cancel (or even if they did, providing the link is helpful), open settings
            // We can check output to see if they clicked "Open Settings", but forcing open is usually fine.
            let _ = Command::new("open")
                .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_ScreenCapture")
                .output();

            return (Vec::new(), Vec::new());
        }
    };


    let displays: Vec<DisplayInfo> = content.displays().iter().enumerate().map(|(i, d)| {
        let id = d.display_id();
        let w = d.width();
        let h = d.height();
        let name = if i == 0 {
            format!("Main Display ({}x{})", w, h)
        } else {
            format!("Display {} ({}x{})", i + 1, w, h)
        };
        DisplayInfo { id, name, width: w, height: h }
    }).collect();

    let windows: Vec<WindowInfo> = content.windows().iter().filter_map(|w| {
        let owner: String = w.owning_application()
            .map(|a| a.application_name())
            .unwrap_or_default();
        let title = w.title().unwrap_or_default();
        let wid = w.window_id();
        let frame = w.frame();
        let width = frame.width as u32;
        let height = frame.height as u32;

        // Filtering for useful windows
        // 1. Must have owner
        // 2. Must be on layer 0 (main app layer)
        // 3. Must be on-screen
        // 4. Must have minimum size
        // 5. If it's a known app with ghost windows (PowerPoint, Chrome), it must have a title
        if owner.is_empty() || w.window_layer() != 0 || !w.is_on_screen() || width < 100 || height < 100 {
            return None;
        }

        // Specifically filter out titleless PowerPoint windows which are often background/utility
        if title.is_empty() && owner.to_lowercase().contains("powerpoint") {
            return None;
        }

        let name = if title.is_empty() {
            owner.clone()
        } else {
            format!("{} – {}", owner, title)
        };

        Some(WindowInfo { id: wid, owner, name, width, height })
    }).collect();

    println!("[SCK] Found {} display(s), {} window(s)", displays.len(), windows.len());
    (displays, windows)
}

// Deprecated individual calls for compatibility if needed, but better to use list_sources
pub fn list_displays() -> Vec<DisplayInfo> {
    list_sources().0
}

pub fn list_windows() -> Vec<WindowInfo> {
    list_sources().1
}


// ========== Frame Extraction Helper ==========

/// Extract BGRA pixel data from a CVPixelBuffer using RAII lock guard.
/// Returns (width, height, data) or None if the buffer is invalid.
fn extract_bgra_from_pixel_buffer(pixel_buffer: &CVPixelBuffer) -> Option<(u32, u32, Vec<u8>)> {
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
            data[dst_offset..dst_offset + copy_len]
                .copy_from_slice(&src_data[src_offset..src_offset + copy_len]);
        }
    }
    
    if data.is_empty() { return None; }

    Some((width, height, data))
    // guard dropped here → auto-unlock
}

// ========== Display Capture ==========

struct DisplayCaptureHandler {
    tx: mpsc::Sender<VideoFrame>,
    stop: Arc<AtomicBool>,
    frame_count: AtomicU64,
}

impl SCStreamOutputTrait for DisplayCaptureHandler {
    fn did_output_sample_buffer(&self, sample: CMSampleBuffer, _of_type: SCStreamOutputType) {
        if self.stop.load(Ordering::Relaxed) { return; }

        if let Some(pixel_buffer) = sample.image_buffer() {
            if let Some((width, height, data)) = extract_bgra_from_pixel_buffer(&pixel_buffer) {
                let frame = VideoFrame {
                    width, height,
                    data: Arc::new(data),
                    timestamp: Instant::now(),
                };

                // Non-blocking send (drop frame if channel full)
                let _ = self.tx.try_send(frame);

                let count = self.frame_count.fetch_add(1, Ordering::Relaxed) + 1;
                if count % 60 == 0 {
                    println!("[SCK] {} frames ({}x{} BGRA, GPU-scaled)", count, width, height);
                }
            }
        }
    }
}

pub fn capture_display(display_id: u32, tx: mpsc::Sender<VideoFrame>, res: ResolutionState) -> StopSignal {
    let stop = StopSignal::new();
    let stop_flag = Arc::new(AtomicBool::new(false));
    let stop_clone = stop.clone();
    let stop_flag_clone = stop_flag.clone();

    println!("[SCK] Starting display capture: ID {}", display_id);

    std::thread::spawn(move || {
        let content = match SCShareableContent::get() {
            Ok(c) => c,
            Err(e) => { eprintln!("[SCK] Error: {:?}", e); return; }
        };

        let display = match content.displays().into_iter().find(|d| d.display_id() == display_id) {
            Some(d) => d,
            None => { eprintln!("[SCK] Display {} not found", display_id); return; }
        };

        // OS does GPU scaling to target resolution!
        let (tw, th) = res.input();
        let (cap_w, cap_h) = if tw > 0 && th > 0 { (tw, th) } else { (display.width(), display.height()) };

        let filter = SCContentFilter::create()
            .with_display(&display)
            .with_excluding_windows(&[])
            .build();

        let config = SCStreamConfiguration::new()
            .with_width(cap_w)
            .with_height(cap_h)
            .with_pixel_format(PixelFormat::BGRA);

        println!("[SCK] Config: {}x{} BGRA (GPU-scaled from native)", cap_w, cap_h);

        let handler = DisplayCaptureHandler {
            tx,
            stop: stop_flag_clone.clone(),
            frame_count: AtomicU64::new(0),
        };

        let mut stream = SCStream::new(&filter, &config);
        stream.add_output_handler(handler, SCStreamOutputType::Screen);

        if let Err(e) = stream.start_capture() {
            eprintln!("[SCK] Failed to start capture: {:?}", e);
            return;
        }

        while !stop_clone.is_stopped() {
            std::thread::sleep(std::time::Duration::from_millis(100));
        }

        stop_flag_clone.store(true, Ordering::Relaxed);
        let _ = stream.stop_capture();
        println!("[SCK] Display capture stopped.");
    });

    stop
}

// ========== Window Capture ==========

struct WindowCaptureHandler {
    tx: mpsc::Sender<VideoFrame>,
    stop: Arc<AtomicBool>,
    frame_count: AtomicU64,
}

impl SCStreamOutputTrait for WindowCaptureHandler {
    fn did_output_sample_buffer(&self, sample: CMSampleBuffer, _of_type: SCStreamOutputType) {
        if self.stop.load(Ordering::Relaxed) { return; }

        if let Some(pixel_buffer) = sample.image_buffer() {
            if let Some((width, height, data)) = extract_bgra_from_pixel_buffer(&pixel_buffer) {
                let frame = VideoFrame {
                    width, height,
                    data: Arc::new(data),
                    timestamp: Instant::now(),
                };

                let _ = self.tx.try_send(frame);

                let count = self.frame_count.fetch_add(1, Ordering::Relaxed) + 1;
                if count % 60 == 0 {
                    println!("[SCK Window] {} frames ({}x{})", count, width, height);
                }
            }
        }
    }
}

pub fn capture_window(window_id: u32, tx: mpsc::Sender<VideoFrame>, res: ResolutionState) -> StopSignal {
    let stop = StopSignal::new();
    let stop_flag = Arc::new(AtomicBool::new(false));
    let stop_clone = stop.clone();
    let stop_flag_clone = stop_flag.clone();

    println!("[SCK] Starting window capture: ID {}", window_id);

    std::thread::spawn(move || {
        let content = match SCShareableContent::get() {
            Ok(c) => c,
            Err(e) => { eprintln!("[SCK] Error: {:?}", e); return; }
        };

        let window = match content.windows().into_iter().find(|w| w.window_id() == window_id) {
            Some(w) => w,
            None => { eprintln!("[SCK] Window {} not found", window_id); return; }
        };

        let (tw, th) = res.input();
        let frame_rect = window.frame();

        let mut cap_w = if tw > 0 && th > 0 {
            tw
        } else {
            frame_rect.width as u32
        };
        let mut cap_h = if tw > 0 && th > 0 {
            th
        } else {
            frame_rect.height as u32
        };

        // ScreenCaptureKit prefers even dimensions
        if cap_w % 2 != 0 { cap_w += 1; }
        if cap_h % 2 != 0 { cap_h += 1; }

        let display = match content.displays().into_iter().next() {
            Some(d) => d,
            None => { eprintln!("[SCK] No display found for window context"); return; }
        };

        let filter = SCContentFilter::create()
            .with_display(&display)
            .with_including_windows(&[&window])
            .build();

        let mut config = SCStreamConfiguration::new();
        config.set_width(cap_w);
        config.set_height(cap_h);
        config.set_pixel_format(PixelFormat::BGRA);
        config.set_shows_cursor(true);
        
        println!("[SCK] Config: {}x{} BGRA window capture", cap_w, cap_h);

        let handler = WindowCaptureHandler {
            tx,
            stop: stop_flag_clone.clone(),
            frame_count: AtomicU64::new(0),
        };

        let mut stream = SCStream::new(&filter, &config);
        stream.add_output_handler(handler, SCStreamOutputType::Screen);

        if let Err(e) = stream.start_capture() {
            eprintln!("[SCK] Failed to start window capture: {:?}", e);
            return;
        }

        while !stop_clone.is_stopped() {
            std::thread::sleep(std::time::Duration::from_millis(100));
        }

        stop_flag_clone.store(true, Ordering::Relaxed);
        let _ = stream.stop_capture();
        println!("[SCK] Window capture stopped.");
    });

    stop
}
