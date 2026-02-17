use crate::ndi::ffi;
use tokio::time::Duration;
use std::{ffi::{CStr, CString}, ptr};
use tokio::sync::mpsc;
use crate::core::{VideoFrame, StopSignal};
use std::sync::{Arc};

pub async fn discover_sources() -> Vec<String> {
    println!("[NDI] Starting discovery...");
    let sources = tokio::task::spawn_blocking(move || {
        let mut found_sources = Vec::new();

        unsafe {
            if !ffi::NDIlib_initialize() {
                eprintln!("[NDI] Failed to init library for discovery");
                return Vec::new();
            }

            let p_ndi_find = ffi::NDIlib_find_create_v2(ptr::null_mut());
            if p_ndi_find.is_null() {
                return Vec::new();
            }

            // Wait for discovery
            std::thread::sleep(Duration::from_secs(2));

            let mut count: u32 = 0;
            let p_sources = ffi::NDIlib_find_get_current_sources(p_ndi_find, &mut count);
            
            // Use count for bounds checking (prevents buffer overread)
            if !p_sources.is_null() && count > 0 {
                for i in 0..count as usize {
                    let src = *p_sources.add(i);
                    if src.p_ndi_name.is_null() { break; }
                    let name = CStr::from_ptr(src.p_ndi_name).to_string_lossy().to_string();
                    found_sources.push(name);
                }
            }

            ffi::NDIlib_find_destroy(p_ndi_find);
            // We should arguably not destroy if we plan to use it effectively, but for "refresh" ensuring clean state is safer
            // ffi::NDIlib_destroy(); // Global destroy might affect other parts? Safe for now if we init every time.
        }
        found_sources
    }).await.unwrap_or_default();
    
    println!("[NDI] Discovered {} sources.", sources.len());
    sources
}

pub fn start_ndi_capture(source_name: String, tx: mpsc::Sender<VideoFrame>, _res: crate::core::ResolutionState) -> StopSignal {
    let stop = StopSignal::new();
    let stop_clone = stop.clone();
    
    println!("[NDI] Connecting to source: {}", source_name);

    std::thread::spawn(move || {
        unsafe {
            if !ffi::NDIlib_initialize() {
                eprintln!("[NDI] Init failed in capture thread");
                return;
            }

            // Create receiver definition
            let source_name_c = CString::new(source_name.clone()).unwrap();
            
            // Use zeroed to handle anonymous unions gracefully
            let mut source_t: ffi::NDIlib_source_t = std::mem::zeroed();
            source_t.p_ndi_name = source_name_c.as_ptr();
            // p_url_address is null by zeroed

            let recv_desc = ffi::NDIlib_recv_create_v3_t {
                source_to_connect_to: source_t,
                color_format: ffi::NDIlib_recv_color_format_e_NDIlib_recv_color_format_BGRX_BGRA,
                bandwidth: ffi::NDIlib_recv_bandwidth_e_NDIlib_recv_bandwidth_highest,
                allow_video_fields: false,
                p_ndi_recv_name: ptr::null(), // Optional custom name
            };

            let p_recv = ffi::NDIlib_recv_create_v3(&recv_desc);
            if p_recv.is_null() {
                eprintln!("[NDI] Failed to create receiver");
                return;
            }

            // CONNECT
            ffi::NDIlib_recv_connect(p_recv, &source_t);
            println!("[NDI] Receiver connected.");

            let mut frame_count = 0;
            // Capture Loop
            while !stop_clone.is_stopped() {
                let mut video_frame: ffi::NDIlib_video_frame_v2_t = std::mem::zeroed();
                let mut audio_frame: ffi::NDIlib_audio_frame_v2_t = std::mem::zeroed();
                let mut metadata_frame: ffi::NDIlib_metadata_frame_t = std::mem::zeroed();

                let frame_type = ffi::NDIlib_recv_capture_v2(
                    p_recv,
                    &mut video_frame,
                    &mut audio_frame,
                    &mut metadata_frame,
                    100 // timeout ms
                );

                match frame_type {
                    ffi::NDIlib_frame_type_e_NDIlib_frame_type_video => {
                        let w = video_frame.xres as u32;
                        let h = video_frame.yres as u32;
                        // Access via anonymous union if needed, or check if direct access works (it failed before)
                        // The error said `__bindgen_anon_1.line_stride_in_bytes`
                        let stride = video_frame.__bindgen_anon_1.line_stride_in_bytes as usize;
                        let data_len = stride * h as usize;
                        let p_data = video_frame.p_data;

                        if !p_data.is_null() {
                            let src_slice = std::slice::from_raw_parts(p_data, data_len);
                            // It's BGRA already due to color_format request
                            // NDI might include padding in stride. 
                             
                            // Ensure packed? BGRA is 4 bytes.
                            // If stride == w * 4, we can copy direct.
                            // Else row-by-row.
                            
                            let mut capture_data = vec![0u8; (w * h * 4) as usize];
                            
                            // Simplification: assume packed for performance if matches, else row-copy
                            if stride == (w * 4) as usize {
                                capture_data.copy_from_slice(src_slice);
                            } else {
                                for y in 0..h {
                                    let s_off = (y as usize) * stride;
                                    let d_off = (y as usize) * (w as usize) * 4;
                                    let copy_len = (w as usize) * 4;
                                    capture_data[d_off..d_off+copy_len].copy_from_slice(
                                        &src_slice[s_off..s_off+copy_len]
                                    );
                                }
                            }

                            // Send frame
                            let frame = VideoFrame {
                                width: w, height: h,
                                data: Some(Arc::new(capture_data)),
                                #[cfg(target_os = "macos")]
                                pixel_buffer: None,
                                timestamp: std::time::Instant::now(),
                            };
                            // Non-blocking: drop frame if compositor can't keep up
                            match tx.try_send(frame) {
                                Ok(_) => {},
                                Err(mpsc::error::TrySendError::Full(_)) => {}, // Drop frame
                                Err(_) => break, // Channel closed
                            }
                            
                            frame_count += 1;
                            if frame_count % 300 == 0 {
                                println!("[NDI] {} frames received ({}x{})", frame_count, w, h);
                            }
                        }
                        
                        ffi::NDIlib_recv_free_video_v2(p_recv, &video_frame);
                    }
                    _ => {} // Ignore audio/metadata/error/none
                }
            }

            // Cleanup — only destroy receiver, NOT global NDI lib
            // (NDI sender thread may still be active)
            println!("[NDI] Stopping capture...");
            ffi::NDIlib_recv_destroy(p_recv);
        }
    });

    stop
}
