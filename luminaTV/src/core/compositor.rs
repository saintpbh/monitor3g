// use wgpu::{Trace, Instance}; // Removed unused
use tokio::sync::mpsc;
use crate::core::{VideoFrame, ResolutionState, scale_rgba_vimage, bgra_to_rgba_vimage};
use crate::core::output_state::OutputState;
use crate::core::key_state::KeyState;
use crate::core::hybrid_key;
use crate::core::blender;
use crate::ndi::sender::NdiSender;
use std::sync::Arc;
use tokio::sync::Mutex;
use std::thread;

/// Shared latest frame for UI preview (zero-copy via Arc)
pub type PreviewFrame = Arc<Mutex<Option<VideoFrame>>>;

/// Hydrate a frame: Ensure it has CPU accessible data (extracts from IOSurface if needed)
fn hydrate_frame(frame: VideoFrame) -> Option<VideoFrame> {
    if frame.data.is_some() {
        return Some(frame);
    }

    #[cfg(target_os = "macos")]
    if let Some(ref pb) = frame.pixel_buffer {
        if let Some((_, _, extracted)) = crate::core::extract_bgra_from_pixel_buffer(pb) {
            return Some(VideoFrame {
                width: frame.width,
                height: frame.height,
                data: Some(Arc::new(extracted)),
                pixel_buffer: frame.pixel_buffer.clone(),
                timestamp: frame.timestamp,
            });
        }
    }

    None
}

/// The Compositor: Handles 5 input layers, composites them, and sends to output.
pub async fn run_compositor(
    mut rx_0: mpsc::Receiver<VideoFrame>,
    mut rx_1: mpsc::Receiver<VideoFrame>,
    mut rx_2: mpsc::Receiver<VideoFrame>,
    mut rx_3: mpsc::Receiver<VideoFrame>,
    mut rx_4: mpsc::Receiver<VideoFrame>,
    output_state: OutputState,
    preview_frame: PreviewFrame,
    res_state: ResolutionState,
    key_state: KeyState,
    layer_key_enables: Vec<Arc<std::sync::atomic::AtomicBool>>,
) {
    println!("[Compositor] Starting 5-Layer compositor loop...");
    init_wgpu().await;

    // === NDI Thread (Blocking) ===
    let (ndi_tx, ndi_rx) = std::sync::mpsc::sync_channel::<VideoFrame>(2);
    thread::spawn(move || {
        let mut sender: Option<NdiSender> = None;
        while let Ok(frame) = ndi_rx.recv() {
            if sender.is_none() {
                sender = NdiSender::new("LuminaTV Output");
            }
            if let Some(ref s) = sender {
                s.send_frame(&frame);
            }
        }
    });

    // === DeckLink Thread (Blocking) ===
    let (dl_tx, dl_rx) = std::sync::mpsc::sync_channel::<VideoFrame>(2);
    thread::spawn(move || {
        use crate::output::decklink::{DeckLinkManager, DeckLinkOutput};
        let mut output: Option<DeckLinkOutput> = None;
        while let Ok(frame) = dl_rx.recv() {
            if output.is_none() {
                let devices = DeckLinkManager::enumerate_devices();
                if !devices.is_empty() {
                    output = DeckLinkOutput::new(&devices[0]);
                }
            }
            if let Some(ref mut out) = output {
                out.send_frame(&frame);
            }
        }
    });

    let mut frame_count: u64 = 0;
    
    // Latest frames for each layer
    // Index 0 = Bottom (Background), Index 4 = Top
    let mut current_frames: [Option<VideoFrame>; 5] = [None, None, None, None, None];
    
    // Alpha history for smoothing (De-flicker)
    let mut previous_alphas: [Option<Vec<u8>>; 5] = [None, None, None, None, None];
    
    // Heartbeat for keeping the compositor running even if inputs are silent
    let mut interval = tokio::time::interval(std::time::Duration::from_millis(33)); // ~30 FPS heartbeat

    loop {
        // === Wait for ANY channel ===
        tokio::select! {
            Some(f) = rx_0.recv() => { 
                if f.width == 0 && f.height == 0 { current_frames[0] = None; } 
                else if let Some(h) = hydrate_frame(f) { current_frames[0] = Some(h); } 
            }
            Some(f) = rx_1.recv() => { 
                if f.width == 0 && f.height == 0 { current_frames[1] = None; } 
                else if let Some(h) = hydrate_frame(f) { current_frames[1] = Some(h); } 
            }
            Some(f) = rx_2.recv() => { 
                if f.width == 0 && f.height == 0 { current_frames[2] = None; } 
                else if let Some(h) = hydrate_frame(f) { current_frames[2] = Some(h); } 
            }
            Some(f) = rx_3.recv() => { 
                if f.width == 0 && f.height == 0 { current_frames[3] = None; } 
                else if let Some(h) = hydrate_frame(f) { current_frames[3] = Some(h); } 
            }
            Some(f) = rx_4.recv() => { 
                if f.width == 0 && f.height == 0 { current_frames[4] = None; } 
                else if let Some(h) = hydrate_frame(f) { current_frames[4] = Some(h); } 
            }
            _ = interval.tick() => {
                // Heartbeat tick - ensures we composite even if no new frames arrived
            }
            else => {
                // All channels closed? Or one closed?
                // tokio::select! with pattern matching handles one branch closing by disabling it?
                // Actually `recv()` returns `None` if closed. We should handle it.
                // But simplified here assuming active channels.
                // If all closed, we should break? 
                // For now, loop continues.
            }
        }

        frame_count += 1;

        // === Composition Pipeline ===
        let (out_w, out_h) = res_state.output();
        if out_w == 0 || out_h == 0 { continue; }

        // Start with black canvas
        let mut composited_data = vec![0u8; (out_w * out_h * 4) as usize];

        // Iterate Bottom -> Top
        for (i, layer_frame_opt) in current_frames.iter().enumerate() {
            if let Some(frame) = layer_frame_opt {
                if let Some(ref data) = frame.data {
                    // 1. Keying (Apply independent key settings per layer? 
                    //    Currently we have ONE KeyState. 
                    //    User requested "Subtitle", "Logo". 
                    //    Usually Logos have alpha (PNG). Subtitles might need keying.
                    //    For now, let's only apply Hybrid Key to Layer 1, 2, 3? 
                    //    Or apply to ALL except Layer 0 (BG)?
                    //    Decision: Apply Key if enabled, to ANY layer except 0?
                    //    Or maybe KeyState should specify which layer it applies to?
                    //    Let's assume Layer 0 is Opaque BG. Layers 1-4 are overlays.
                    //    We will apply Hybrid Key to Layers 1-4 if Key is enabled globally.
                    //    (Ideal: Each layer has its own key setting, but that's complex UI).
                    
                    // 1. Keying
                    // Apply if GLOBAL key is enabled AND LAYER key is enabled
                    let layer_key_on = layer_key_enables[i].load(std::sync::atomic::Ordering::Relaxed);
                    
                    let working_data = if layer_key_on {
                         let mut d = data.to_vec();
                         let pixel_count = (frame.width * frame.height) as usize;
                         
                         // Manage previous alpha buffer size
                         if previous_alphas[i].as_ref().map_or(true, |v| v.len() != pixel_count) {
                             previous_alphas[i] = Some(vec![255; pixel_count]); // Init opaque? Or 0?
                             // If we init 255, smooth transition might fade in. 
                             // If we init 0, might fade out.
                             // Let's init 0 (transparent) so it fades in.
                             // Actually, 0 means previous was fully transparent.
                             previous_alphas[i] = Some(vec![0; pixel_count]);
                         }

                         // Note: Keying uses original resolution
                         hybrid_key::apply_hybrid_key(
                             &mut d, 
                             frame.width, 
                             frame.height, 
                             &key_state,
                             previous_alphas[i].as_mut().map(|v| v.as_mut_slice())
                         );
                         d
                    } else {
                         // Reset smoothing buffer if key disabled?
                         // If we re-enable, we don't want old data.
                         if previous_alphas[i].is_some() { previous_alphas[i] = None; }
                         data.to_vec()
                    };

                    // 2. Scale to Output
                    let scaled_data = if frame.width != out_w || frame.height != out_h {
                        // scale_rgba_vimage takes slice, returns Vec
                        scale_rgba_vimage(&working_data, frame.width, frame.height, out_w, out_h)
                    } else {
                        working_data
                    };

                    // 3. Blend onto canvas
                    if i == 0 {
                        // Layer 0 is base, just copy (overwrite black)
                        composited_data = scaled_data; 
                        // Ensure alpha is 255 for base? scale_rgba usually preserves.
                        // But composite output must be opaque for NDI/DeckLink usually (or Premultiplied).
                        // Let's ensure opaque alpha for Layer 0 if it's "Background".
                        // Actually blender::blend_frames handles blending ONTO something.
                        // If composited_data is initialized black (0,0,0,0), blending opaque image works.
                    } else {
                        blender::blend_frames(&mut composited_data, &scaled_data, out_w, out_h);
                    }
                }
            }
        }

        let output_frame = VideoFrame {
            width: out_w, height: out_h,
            data: Some(Arc::new(composited_data)),
            #[cfg(target_os = "macos")]
            pixel_buffer: None,
            timestamp: std::time::Instant::now(),
        };

        // === OUTPUT ===
        if output_state.is_ndi_enabled() {
             let _ = ndi_tx.try_send(output_frame.clone());
        }
        if output_state.is_decklink_enabled() {
            let _ = dl_tx.try_send(output_frame.clone());
        }

        // === PREVIEW (Half-Size) ===
        if let Ok(mut preview) = preview_frame.try_lock() {
            let pw = out_w / 2;
            let ph = out_h / 2;
            if pw > 0 && ph > 0 {
                let mut preview_data = scale_rgba_vimage(output_frame.data.as_ref().unwrap(), out_w, out_h, pw, ph);
                bgra_to_rgba_vimage(&mut preview_data, pw, ph);
                
                *preview = Some(VideoFrame {
                    width: pw, height: ph,
                    data: Some(Arc::new(preview_data)),
                    #[cfg(target_os = "macos")]
                    pixel_buffer: None,
                    timestamp: output_frame.timestamp,
                });
            }
        }
        
        // Log every 300 frames
        if frame_count % 300 == 0 {
             let status: String = current_frames.iter().enumerate()
                .map(|(i, f)| if f.is_some() { format!("L{}:ON", i) } else { format!("L{}:-", i) })
                .collect::<Vec<_>>().join(" ");
             println!("[Compositor] #{} Status: {}", frame_count, status);
        }
    }
}

pub async fn init_wgpu() {
    println!("Initializing wgpu (placeholder for future use)...");
}
