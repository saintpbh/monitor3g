use wgpu::{DeviceDescriptor, ExperimentalFeatures, Trace, Instance};
use tokio::sync::mpsc;
use crate::core::{VideoFrame, ResolutionState, scale_rgba_vimage, bgra_to_rgba_vimage};
use crate::core::output_state::OutputState;
use crate::ndi::sender::NdiSender;
use std::sync::Arc;
use tokio::sync::Mutex;
use std::thread;

/// Shared latest frame for UI preview (zero-copy via Arc)
pub type PreviewFrame = Arc<Mutex<Option<VideoFrame>>>;

pub async fn run_compositor(
    mut receiver: mpsc::Receiver<VideoFrame>,
    output_state: OutputState,
    preview_frame: PreviewFrame,
    res_state: ResolutionState,
) {
    println!("[Compositor] Starting compositor loop...");
    init_wgpu().await;

    // === NDI Thread (Blocking) ===
    // We use a small sync_channel to drop frames if NDI is too slow.
    // This ensures the main compositor loop (and UI preview) never blocks.
    let (ndi_tx, ndi_rx) = std::sync::mpsc::sync_channel::<VideoFrame>(2);
    
    // Spawn dedicated OS thread for NDI sending (blocking FFI)
    thread::spawn(move || {
        let mut sender: Option<NdiSender> = None;
        println!("[NDI Thread] Started.");

        while let Ok(frame) = ndi_rx.recv() {
            if sender.is_none() {
                println!("[NDI Thread] Initializing sender...");
                sender = NdiSender::new("LuminaTV Output");
            }
            if let Some(ref s) = sender {
                s.send_frame(&frame);
            }
        }
        println!("[NDI Thread] Stopped.");
    });

    // === DeckLink Thread (Blocking) ===
    let (dl_tx, dl_rx) = std::sync::mpsc::sync_channel::<VideoFrame>(2);
    thread::spawn(move || {
        use crate::output::decklink::{DeckLinkManager, DeckLinkOutput};
        let mut output: Option<DeckLinkOutput> = None;
        println!("[DeckLink Thread] Started.");

        while let Ok(frame) = dl_rx.recv() {
            if output.is_none() {
                println!("[DeckLink Thread] Searching for devices...");
                let devices = DeckLinkManager::enumerate_devices();
                if !devices.is_empty() {
                    println!("[DeckLink Thread] Selecting first device: {}", devices[0].name);
                    output = DeckLinkOutput::new(&devices[0]);
                }
            }
            if let Some(ref mut out) = output {
                out.send_frame(&frame);
            }
        }
        println!("[DeckLink Thread] Stopped.");
    });

    let mut frame_count: u64 = 0;

    loop {
        // === LATEST-FRAME-ONLY: drain channel, keep only newest ===
        let first = match receiver.recv().await {
            Some(f) => f,
            None => break,
        };
        let mut latest = first;
        while let Ok(f) = receiver.try_recv() {
            latest = f;
        }
        let frame = latest;
        frame_count += 1;

        // === HYDRATION: Ensure we have CPU data (Lazy Extraction) ===
        // If the frame came from mac.rs (Zero-Copy), it has no data yet.
        // We extract it here so downstream (NDI, DeckLink, Preview) can use it.
        let data_ref = if let Some(ref d) = frame.data {
             d.clone()
        } else {
             #[cfg(target_os = "macos")]
             if let Some(ref pb) = frame.pixel_buffer {
                 if let Some((_, _, extracted)) = crate::core::extract_bgra_from_pixel_buffer(pb) {
                     Arc::new(extracted)
                 } else {
                     eprintln!("[Compositor] Failed to extract from pixel buffer");
                     continue;
                 }
             } else {
                 continue; // No data source
             }
             #[cfg(not(target_os = "macos"))]
             continue;
        };

        // Create a wrapper frame that definitely has data
        let hydrated_frame = VideoFrame {
            width: frame.width, height: frame.height,
            data: Some(data_ref.clone()), // Share the Arc
            #[cfg(target_os = "macos")]
            pixel_buffer: frame.pixel_buffer.clone(),
            timestamp: frame.timestamp,
        };
        // Use hydrated_frame for the rest of the loop
        let frame = hydrated_frame;

        // === Output: BGRA data goes DIRECTLY to NDI (zero copy/conversion!) ===
        let (out_w, out_h) = res_state.output();
        let output_frame = if out_w > 0 && out_h > 0 && (out_w != frame.width || out_h != frame.height) {
            // Output needs different resolution — scale BGRA data
            // We use .unwrap() because we just hydrated it above
            let scaled = scale_rgba_vimage(frame.data.as_ref().unwrap(), frame.width, frame.height, out_w, out_h);
            VideoFrame {
                width: out_w, height: out_h,
                data: Some(Arc::new(scaled)),
                #[cfg(target_os = "macos")]
                pixel_buffer: None,
                timestamp: frame.timestamp,
            }
        } else {
            frame.clone()
        };

        // === FIRE-AND-FORGET NDI ===
        if output_state.is_ndi_enabled() {
             let _ = ndi_tx.try_send(output_frame.clone());
        }

        // === FIRE-AND-FORGET DeckLink ===
        if output_state.is_decklink_enabled() {
            let _ = dl_tx.try_send(output_frame.clone());
        }

        // === Preview: Scaled to match OUTPUT resolution (half-size) + BGRA→RGBA for Slint ===
        if let Ok(mut preview) = preview_frame.try_lock() {
            // Preview should match the OUTPUT aspect ratio, not the input frame.
            // This ensures the preview shows exactly what NDI/DeckLink outputs.
            let pw = if out_w > 0 { out_w / 2 } else { frame.width / 2 };
            let ph = if out_h > 0 { out_h / 2 } else { frame.height / 2 };

            if pw > 0 && ph > 0 {
                // Scale from the output_frame (already scaled to output res) to preview size
                let preview_data = if let Some(ref d) = output_frame.data {
                    let mut scaled = if pw != output_frame.width || ph != output_frame.height {
                        scale_rgba_vimage(d, output_frame.width, output_frame.height, pw, ph)
                    } else {
                        d.to_vec()
                    };
                    bgra_to_rgba_vimage(&mut scaled, pw, ph);
                    Some(scaled)
                } else {
                    None
                };

                if let Some(d) = preview_data {
                    *preview = Some(VideoFrame {
                        width: pw, height: ph,
                        data: Some(Arc::new(d)),
                        #[cfg(target_os = "macos")]
                        pixel_buffer: None,
                        timestamp: frame.timestamp,
                    });
                }
            }
        }

        // Latency + status logging
        if frame_count % 300 == 0 {
            let latency = frame.timestamp.elapsed();
            let ndi_s = if output_state.is_ndi_enabled() { "ON" } else { "OFF" };
            let dl_s = if output_state.is_decklink_enabled() { "ON" } else { "OFF" };
            println!(
                "[Compositor] #{}: {}x{} BGRA → out {}x{} | preview {}x{} RGBA | NDI:{} DL:{} | {:.1}ms",
                frame_count, frame.width, frame.height,
                out_w, out_h,
                frame.width / 2, frame.height / 2,
                ndi_s, dl_s,
                latency.as_secs_f64() * 1000.0
            );
        }
    }
}

pub async fn init_wgpu() {
    println!("Initializing wgpu...");
    let instance = Instance::default();
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            force_fallback_adapter: false,
            compatible_surface: None,
        })
        .await
        .expect("Failed to find adapter");
    println!("Found adapter: {}", adapter.get_info().name);

    let (_device, _queue) = adapter
        .request_device(&DeviceDescriptor {
            label: None,
            required_features: wgpu::Features::empty(),
            required_limits: wgpu::Limits::default(),
            memory_hints: wgpu::MemoryHints::default(),
            experimental_features: ExperimentalFeatures::disabled(),
            trace: Trace::Off,
        })
        .await
        .expect("Failed to create device");
    println!("Created wgpu device and queue.");
}
