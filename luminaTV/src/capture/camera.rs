use nokhwa::utils::{ApiBackend, CameraIndex, RequestedFormat, RequestedFormatType, CameraInfo};

use nokhwa::pixel_format::RgbAFormat;
use nokhwa::{query, Camera};
use tokio::sync::mpsc;
use crate::core::{VideoFrame, StopSignal, ResolutionState, scale_rgba_vimage};
use std::sync::Arc;


#[cfg(target_os = "macos")]
#[path = "mac_avf.rs"]
mod mac_avf;

#[cfg(not(target_os = "macos"))]
pub fn list_cameras() -> Vec<(usize, String)> {
    match query(ApiBackend::Auto) {
        Ok(devices) => {
            devices.iter().enumerate().map(|(i, d)| (i, d.human_name().to_string())).collect()
        }
        Err(e) => {
            eprintln!("[Camera] Failed to query devices: {}", e);
            Vec::new()
        }
    }
}

// Duplicate imports removed


#[cfg(target_os = "macos")]
pub fn list_cameras() -> Vec<(usize, String)> {
    let cameras = match query(ApiBackend::Auto) {
        Ok(devices) => {
             devices.iter().enumerate().map(|(i, d): (usize, &CameraInfo)| {
                (i, d.human_name().to_string()) 
             }).collect::<Vec<(usize, String)>>()
        },
        Err(e) => {
            eprintln!("[Camera] Failed to list: {}", e);
            Vec::new()
        }
    };
    println!("[Camera] Found {} camera(s)", cameras.len());
    cameras
}


#[cfg(target_os = "macos")]
pub fn start_camera(camera_idx: usize, tx: mpsc::Sender<VideoFrame>, _res: ResolutionState) -> StopSignal {
    let stop = StopSignal::new();
    let stop_clone = stop.clone();

    // We need the device unique ID.
    // Re-query to find it.
    let devices: Vec<nokhwa::utils::CameraInfo> = match query(ApiBackend::AVFoundation) {
        Ok(d) => d,
        Err(_) => return stop,
    };

    
    if camera_idx >= devices.len() { return stop; }
    let info = &devices[camera_idx];
    
    // Nokhwa's misc() often holds the unique ID if index() is not it.
    // Or `info.index()` might be the unique ID string wrapped.
    let unique_id = match info.index() {
        CameraIndex::String(s) => s.clone(),
        _ => {
            eprintln!("[Camera] AVF requires string ID, got {:?}", info.index());
            return stop;
        }
    };

    println!("[Camera] Starting Native AVF Capture: {} ({})", info.human_name(), unique_id);

    std::thread::spawn(move || {
        // Create session
        // Note: AVF objects must be created/used typically on threads compatible with runloops or dispatch queues.
        // Our `mac_avf` creates its own queue, so it should be fine.
        
        let session = match mac_avf::CameraSession::start(&unique_id, tx) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("[Camera] AVF Init Error: {}", e);
                return;
            }
        };
        
        // Wait for stop signal
        while !stop_clone.is_stopped() {
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
        
        println!("[Camera] Stopping AVF session...");
        session.stop();
    });

    stop
}

#[cfg(not(target_os = "macos"))]
pub fn start_camera(camera_idx: usize, tx: mpsc::Sender<VideoFrame>, res: ResolutionState) -> StopSignal {

    let stop = StopSignal::new();
    let stop_clone = stop.clone();

    let cameras = match query(ApiBackend::Auto) {
        Ok(c) => c,
        Err(e) => { eprintln!("[Camera] Query failed: {}", e); return stop; }
    };

    if camera_idx >= cameras.len() {
        eprintln!("[Camera] Index {} out of range ({})", camera_idx, cameras.len());
        return stop;
    }

    let index = cameras[camera_idx].index().clone();
    let name = cameras[camera_idx].human_name().to_string();
    println!("[Camera] Starting capture: {} (index {})", name, camera_idx);

    std::thread::spawn(move || {
        if let Err(e) = capture_loop(index, tx, stop_clone, res) {
            eprintln!("[Camera] Error: {}", e);
        }
    });

    stop
}

fn capture_loop(
    index: CameraIndex,
    tx: mpsc::Sender<VideoFrame>,
    stop: StopSignal,
    res: ResolutionState,
) -> Result<(), Box<dyn std::error::Error>> {
    // 1. Open with lenient default (AbsoluteHighestFrameRate) to ensure success
    let mut camera = Camera::new(index, RequestedFormat::new::<RgbAFormat>(
        RequestedFormatType::AbsoluteHighestFrameRate
    ))?;
    camera.open_stream()?;

    // 2. Smart upgrade to 1080p/720p if possible
    let current_fmt = camera.camera_format();
    println!("[Camera] Initial: {} ({}x{} @ {})", 
        current_fmt.format(), current_fmt.width(), current_fmt.height(), current_fmt.frame_rate());

    if let Ok(supported) = camera.compatible_camera_formats() {
        let mut best_fmt = None;
        let mut max_score = 0;

        for fmt in supported {
            let w: u32 = fmt.width();

            let h = fmt.height();
            let fps = fmt.frame_rate();
            
            // Filter: require at least 15fps, favor MJPEG/YUYV if strictly needed but sticking to resolution score
            if fps < 15 { continue; }
            
            // Score preferences: 
            // 1. Resolution match (1920x1080 > 1280x720 > others)
            // 2. High FPS
            // 3. MJPEG usually preferred for USB bandwidth
            
            let mut score = w * h * fps;
            
            // Boost 1080p/720p specifically
            if w == 1920 && h == 1080 { score *= 10; }
            if w == 1280 && h == 720 { score *= 5; }
            
            if score > max_score {
                max_score = score;
                best_fmt = Some(fmt);
            }
        }

        if let Some(target) = best_fmt {
             // Only switch if different and "better" (or specifically target)
             if target.width() != current_fmt.width() || target.height() != current_fmt.height() {
                println!("[Camera] Upgrading to: {}x{} @ {}", 
                    target.width(), target.height(), target.frame_rate());
                
                // Re-open stream with new format
                let _ = camera.stop_stream();
                // set_camera_requset takes RequestedFormat, not CameraFormat directly? 
                // Using set_camera_format for exact match (even if deprecated, it works for CameraFormat)
                #[allow(deprecated)]
                let _ = camera.set_camera_format(target);
                let _ = camera.open_stream();
             }
        }
    }

    let resolution = camera.resolution();
    println!("[Camera] Final: {}x{}", resolution.width(), resolution.height());

    let mut frame_count: u64 = 0;
    while !stop.is_stopped() {
        match camera.frame() {
            Ok(buffer) => {
                let decoded: image::ImageBuffer<image::Rgba<u8>, Vec<u8>> = buffer.decode_image::<RgbAFormat>()
                    .map_err(|e| format!("decode: {}", e))?;



                let src_w = decoded.width();
                let src_h = decoded.height();
                let raw = decoded.into_raw();

                let (tw, th) = res.input();
                let (out_w, out_h, data) = if tw > 0 && th > 0 && (tw != src_w || th != src_h) {
                    let scaled = scale_rgba_vimage(&raw, src_w, src_h, tw, th);
                    (tw, th, scaled)
                } else {
                    (src_w, src_h, raw)
                };

                let frame = VideoFrame {
                    width: out_w, height: out_h,
                    data: Arc::new(data),
                    timestamp: std::time::Instant::now(),
                };

                // Non-blocking send: drop frame if compositor is busy
                match tx.try_send(frame) {
                    Ok(_) => {},
                    Err(mpsc::error::TrySendError::Full(_)) => {
                        // Drop frame, don't block capture
                    },
                    Err(_) => break, // Channel closed
                }

                frame_count += 1;
                if frame_count % 300 == 0 {
                    println!("[Camera] {} frames ({}x{} → {}x{})", frame_count, src_w, src_h, out_w, out_h);
                }
            }
            Err(e) => {
                if stop.is_stopped() { break; }
                // Don't spam logs on minor timeouts/errors
                if frame_count % 60 == 0 {
                     eprintln!("[Camera] Frame error: {}", e);
                }
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
        }
    }
    println!("[Camera] Stopped.");
    Ok(())

}

