/// ScreenCaptureKit-based screen capture (macOS 12.3+)
/// Zero-copy GPU pipeline: SCStream → IOSurface → CVPixelBuffer → BGRA data
///
/// Key advantage: ScreenCaptureKit does hardware GPU scaling at the OS level.
/// We request the target resolution via SCStreamConfiguration, and the OS
/// delivers frames already scaled — no vImage or CPU scaling needed.

use screencapturekit::prelude::*;
use screencapturekit::cm::CMSampleBuffer;
use tokio::sync::mpsc;
use crate::core::{VideoFrame, StopSignal, ResolutionState};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

// Re-export types for main.rs compatibility
pub struct DisplayInfo {
    pub id: u32,
    pub name: String,
    pub width: u32,
    pub height: u32,
}

#[derive(Clone, Debug)]
pub struct WindowInfo {
    pub id: u32,
    pub owner: String,
    pub name: String,
    pub width: u32,
    pub height: u32,
    pub is_presentation: bool,
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

    let mut windows: Vec<WindowInfo> = content.windows().iter().filter_map(|w| {
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
        // 3. Must be on-screen (RELAXED for PowerPoint as presentation might be on another space/off-screen)
        // 4. Must have minimum size (100x100)
        
        let is_ppt = owner.to_lowercase().contains("powerpoint");
        
        // Debug Log for PPT
        if is_ppt {
            println!("[SCK DEBUG] PPT Window: '{}' (ID {}) Layer: {} OnScreen: {} Size: {}x{}", 
                title, wid, w.window_layer(), w.is_on_screen(), width, height);
        }

        // Basic validity and size checks
        if owner.is_empty() || w.window_layer() != 0 || width < 50 || height < 50 {
             return None;
        }

        // Specific filtering for PowerPoint garbage
        if is_ppt {
            // Filter out menu bars/toolbars (very wide, very short)
            if height < 100 { 
                return None; 
            }
            // Filter out small square-ish icons/tools
            if width < 200 {
                return None;
            }
            // Filter out ALL untitled PowerPoint windows (based on logs, useful ones have titles)
            if title.is_empty() {
                return None;
            }
        }

        // On-screen check: Strict for most apps, relaxed for PPT
        if !is_ppt && !w.is_on_screen() {
            return None;
        }

        // Relaxed on-screen check for PPT
        if is_ppt && !w.is_on_screen() {
             // Ensure it's not tiny (though title check might cover this)
             if width < 500 || height < 400 {
                 return None;
             }
        }

        let name = if title.is_empty() {
             format!("{} (Window {})", owner, wid)
        } else {
            // Detect Slide Show
            if is_ppt && (title.contains("Slide Show") || title.contains("슬라이드 쇼")) {
                format!("PowerPoint Slide Show ({})", title)
            } else {
                format!("{} – {}", owner, title)
            }
        };

        let is_presentation = is_ppt && (name.contains("Slide Show") || name.contains("슬라이드 쇼") || name.contains("Presenting"));

        Some(WindowInfo { id: wid, owner, name, width, height, is_presentation })
    }).collect();

    // Sort: Presentations first!
    windows.sort_by(|a, b| b.is_presentation.cmp(&a.is_presentation));

    // println!("[SCK] Found {} display(s), {} window(s)", displays.len(), windows.len());
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


// ========== Display Capture ==========

struct DisplayCaptureHandler {
    tx: mpsc::Sender<VideoFrame>,
    stop: StopSignal,
    frame_count: AtomicU64,
}

impl SCStreamOutputTrait for DisplayCaptureHandler {
    fn did_output_sample_buffer(&self, sample: CMSampleBuffer, _of_type: SCStreamOutputType) {
        if self.stop.is_stopped() { return; }

        if let Some(pixel_buffer) = sample.image_buffer() {
            // ARC the pixel buffer for sharing
            let pixel_buffer_arc = Arc::new(pixel_buffer);
            let width = pixel_buffer_arc.width() as u32;
            let height = pixel_buffer_arc.height() as u32;

            // LAZY EXTRACTION: skip CPU copy!
            let frame = VideoFrame {
                width, height,
                data: None, // Lazy extract in Compositor if needed
                #[cfg(target_os = "macos")]
                pixel_buffer: Some(pixel_buffer_arc.clone()),
                timestamp: Instant::now(),
            };

            // Non-blocking send (drop frame if channel full)
            let _ = self.tx.try_send(frame);

            let count = self.frame_count.fetch_add(1, Ordering::Relaxed) + 1;
            if count % 60 == 0 {
                println!("[SCK] {} frames ({}x{}) - Zero Copy Mode", count, width, height);
            }
        }
    }
}

pub fn capture_display(display_id: u32, tx: mpsc::Sender<VideoFrame>, res: ResolutionState) -> StopSignal {
    let stop = StopSignal::new();
    let stop_clone = stop.clone();
    let stop_for_handler = stop.clone();

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
            stop: stop_for_handler,
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

        let _ = stream.stop_capture();
        println!("[SCK] Display capture stopped.");
    });

    stop
}

// ========== Window Capture ==========

struct WindowCaptureHandler {
    tx: mpsc::Sender<VideoFrame>,
    stop: StopSignal,
    frame_count: AtomicU64,
}

impl SCStreamOutputTrait for WindowCaptureHandler {
    fn did_output_sample_buffer(&self, sample: CMSampleBuffer, _of_type: SCStreamOutputType) {
        if self.stop.is_stopped() { return; }

        if let Some(pixel_buffer) = sample.image_buffer() {
            // ARC the pixel buffer for sharing
            let pixel_buffer_arc = Arc::new(pixel_buffer);
            let width = pixel_buffer_arc.width() as u32;
            let height = pixel_buffer_arc.height() as u32;

            let frame = VideoFrame {
                width, height,
                data: None,
                #[cfg(target_os = "macos")]
                pixel_buffer: Some(pixel_buffer_arc.clone()),
                timestamp: Instant::now(),
            };

            match self.tx.try_send(frame) {
                Ok(_) => {}
                Err(mpsc::error::TrySendError::Full(_)) => {}
                Err(e) => {
                         eprintln!("[SCK Window] Channel closed: {:?}", e);
                }
            }

            let count = self.frame_count.fetch_add(1, Ordering::Relaxed) + 1;
            if count % 60 == 0 {
                println!("[SCK Window] {} frames ({}x{}) - Zero Copy Mode", count, width, height);
            }
        }
    }
}

pub fn capture_window(window_id: u32, tx: mpsc::Sender<VideoFrame>, _res: ResolutionState) -> StopSignal {
    let stop = StopSignal::new();
    let stop_clone = stop.clone();
    let stop_for_handler = stop.clone();

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

        let frame_rect = window.frame();

        // IMPORTANT: Always capture at the window's native size!
        // ScreenCaptureKit renders the window at its native resolution within the buffer.
        // If we request a larger buffer (e.g. 1920x1080 for a 854x540 window),
        // the content floats in a corner with black padding.
        // By capturing at native size, the content fills the entire frame.
        // The compositor then scales this to the output resolution for full-screen output.
        let mut cap_w = frame_rect.width as u32;
        let mut cap_h = frame_rect.height as u32;

        // Minimum size guard
        if cap_w == 0 { cap_w = 1920; }
        if cap_h == 0 { cap_h = 1080; }

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
            stop: stop_for_handler,
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

        let _ = stream.stop_capture();
        println!("[SCK] Window capture stopped.");
    });

    stop
}
