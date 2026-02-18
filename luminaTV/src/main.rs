#[cxx::bridge]
mod ffi {
    unsafe extern "C++" {
        include!("luminaTV/cxx/bridge.h");
    }
}

pub mod core;
pub mod capture;
pub mod ndi;
pub mod output;
pub mod ui;

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::sync::Mutex;
use crate::core::{StopSignal, ResolutionState, RESOLUTION_PRESETS};

slint::include_modules!();

struct LayerState {
    active_stop: Option<StopSignal>,
    current_type: String,
    current_source_name: String,
    tx: Option<tokio::sync::mpsc::Sender<core::VideoFrame>>,
}

impl LayerState {
    fn new(tx: tokio::sync::mpsc::Sender<core::VideoFrame>) -> Self {
        Self {
            active_stop: None,
            current_type: "None".to_string(), // Default type is None
            current_source_name: "No Source".to_string(),
            tx: Some(tx),
        }
    }
}

// Helper to stop a layer
fn stop_layer(layer: &mut LayerState) {
    if let Some(ref stop) = layer.active_stop {
        stop.stop();
    }
    // Send clear frame if tx exists
    if let Some(ref tx) = layer.tx {
        let _ = tx.try_send(core::VideoFrame {
             width: 0, height: 0, 
             data: None, 
             #[cfg(target_os = "macos")] pixel_buffer: None, 
             timestamp: std::time::Instant::now() 
        });
    }
    layer.active_stop = None;
    layer.current_source_name = "No Source".to_string();
}

struct SourceState {
    // Global Source Lists
    cameras: Vec<(usize, String)>,
    displays: Vec<capture::mac::DisplayInfo>,
    windows: Vec<capture::mac::WindowInfo>,
    ndi_sources: Vec<String>,
    
    // 5 Layers
    layers: Vec<LayerState>, // Size 5
    selected_layer_idx: usize,

    virtual_display: Option<capture::VirtualDisplay>,
    decklink_input: Option<output::decklink_input::DeckLinkInput>,
    monitor_rx: Option<std::sync::mpsc::Receiver<core::VideoFrame>>,
}

#[tokio::main]
async fn main() -> Result<(), slint::PlatformError> {
    println!("LuminaTV starting...");

    #[cfg(target_os = "macos")]
    capture::camera::request_permission();

    let output_state = core::output_state::OutputState::new();
    let preview_frame: core::compositor::PreviewFrame = Arc::new(Mutex::new(None));
    let res_state = ResolutionState::new();
    let key_state = core::key_state::KeyState::new();

    let ui = AppWindow::new()?;

    // Enumerate sources
    println!("[Main] Enumerating cameras...");
    let cameras = capture::camera::list_cameras();
    
    println!("[Main] Enumerating sources (Displays & Windows)...");
    let (displays, windows) = capture::list_sources();

    println!("[Main] Startup: Found {} cameras, {} displays, {} windows", 
        cameras.len(), displays.len(), windows.len());


    // Create 5 Channels for 5 Layers
    let (tx0, rx0) = tokio::sync::mpsc::channel(4);
    let (tx1, rx1) = tokio::sync::mpsc::channel(4);
    let (tx2, rx2) = tokio::sync::mpsc::channel(4);
    let (tx3, rx3) = tokio::sync::mpsc::channel(4);
    let (tx4, rx4) = tokio::sync::mpsc::channel(4);

    let layers = vec![
        LayerState::new(tx0),
        LayerState::new(tx1),
        LayerState::new(tx2),
        LayerState::new(tx3),
        LayerState::new(tx4),
    ];

    // Shared Key Enable State per Layer
    let layer_key_enables: Vec<Arc<AtomicBool>> = (0..5).map(|_| Arc::new(AtomicBool::new(false))).collect();
    let layer_key_enables_comp = layer_key_enables.clone();
    let layer_key_enables_ui = layer_key_enables.clone();

    let source_state = Arc::new(std::sync::Mutex::new(SourceState {
        cameras,
        displays,
        windows,
        ndi_sources: Vec::new(),
        layers,
        selected_layer_idx: 0,
        virtual_display: None,
        decklink_input: None,
        monitor_rx: None,
    }));

    // Start Compositor
    let output_state_comp = output_state.clone();
    let preview_frame_comp = preview_frame.clone();
    let res_comp = res_state.clone();
    let key_comp = key_state.clone();
    tokio::spawn(async move {
        core::compositor::run_compositor(
            rx0, rx1, rx2, rx3, rx4,
            output_state_comp, preview_frame_comp, res_comp, key_comp,
            layer_key_enables_comp
        ).await;
    });

    // UltraStudio / DeckLink Enumeration
    tokio::spawn(async move {
        println!("[Main] Enumerating DeckLink devices...");
        let devices = output::decklink::DeckLinkManager::enumerate_devices();
        println!("[Main] Found {} DeckLink devices.", devices.len());
        for d in devices {
             println!("[Main] - {} (Index: {})", d.name, d.index);
        }
    });

    // Initialize UI with Layer 0 info
    {
         // Default Layer 0 is "Screen", others are "None"
         // actually we set default in LayerState::new to None.
         // But we want Layer 0 to start as Screen.
         let mut ss = source_state.lock().unwrap();
         ss.layers[0].current_type = "None".to_string();

         let names: Vec<slint::SharedString> = vec![]; // Start empty for None
         let model = std::rc::Rc::new(slint::VecModel::from(names));
         ui.set_device_list(slint::ModelRc::from(model));
         ui.set_selected_source_type("None".into());
         
         // Set Layer 1 active, others inactive
         let active_flags: Vec<bool> = vec![true, false, false, false, false];
         let active_model = std::rc::Rc::new(slint::VecModel::from(active_flags));
         ui.set_layer_active(slint::ModelRc::from(active_model));
    }

    // === Resolution Callbacks ===
    let res_input = res_state.clone();
    ui.on_select_input_resolution(move |idx| {
        let idx = idx as usize;
        if idx < RESOLUTION_PRESETS.len() {
            let p = &RESOLUTION_PRESETS[idx];
            res_input.set_input(p.width, p.height);
        }
    });

    let res_output = res_state.clone();
    ui.on_select_output_resolution(move |idx| {
        let idx = idx as usize;
        if idx < RESOLUTION_PRESETS.len() {
            let p = &RESOLUTION_PRESETS[idx];
            res_output.set_output(p.width, p.height);
        }
    });

    // === Source Callbacks ===

    // 1. Select Layer
    let ui_handle_layer = ui.as_weak();
    let ss_layer = source_state.clone();
    ui.on_select_layer(move |idx| {
        let idx = idx as usize;
        if idx >= 5 { return; }
        
        println!("[UI] Selected Layer: {}", idx);
        
        let mut ss = match ss_layer.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        ss.selected_layer_idx = idx;
        let current_type = ss.layers[idx].current_type.clone();
        let current_name = ss.layers[idx].current_source_name.clone();
        
        // Update UI
        if let Some(ui) = ui_handle_layer.upgrade() {
            ui.set_selected_source_type(slint::SharedString::from(current_type.as_str()));
            ui.set_source_name(slint::SharedString::from(current_name.as_str()));
            
            // Populate list for this type
             let names: Vec<slint::SharedString> = match current_type.as_str() {
                "Screen" => ss.displays.iter().map(|d| slint::SharedString::from(d.name.as_str())).collect(),
                "Window" => ss.windows.iter().map(|w| slint::SharedString::from(w.name.as_str())).collect(),
                "Camera" => ss.cameras.iter().map(|(_, n)| slint::SharedString::from(n.as_str())).collect(),
                "NDI" => ss.ndi_sources.iter().map(|n| slint::SharedString::from(n.as_str())).collect(),
                "Presentation" => {
                    let p_wins: Vec<_> = ss.windows.iter().filter(|w| w.is_presentation).collect();
                    if p_wins.is_empty() {
                         vec![slint::SharedString::from("Waiting for Slide Show...")]
                    } else {
                         p_wins.iter().map(|w| slint::SharedString::from(format!("READY: {}", w.name).as_str())).collect()
                    }
                },
                _ => vec![],
            };
            let model = std::rc::Rc::new(slint::VecModel::from(names));
            ui.set_device_list(slint::ModelRc::from(model));
        }
    });

    // 2. Select Source Type
    let ui_handle_type = ui.as_weak();
    let ss_type = source_state.clone();
    ui.on_select_source_type(move |source_type| {
        let st = source_type.to_string();
        println!("[UI] Source type changed to: {}", st);

        let mut ss = match ss_type.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner()
        };
        
        let idx = ss.selected_layer_idx;
        ss.layers[idx].current_type = st.clone();

        // Update Device List
        if let Some(ui) = ui_handle_type.upgrade() {
            let names: Vec<slint::SharedString> = match st.as_str() {
                "None" => {
                    // Stop layer immediately
                    stop_layer(&mut ss.layers[idx]);
                    ui.set_source_name("No Source".into());
                    
                    // Update Active Status (Inactive)
                    let mut flags = Vec::new();
                    for l in &ss.layers { flags.push(l.active_stop.is_some()); }
                    let model = std::rc::Rc::new(slint::VecModel::from(flags));
                    ui.set_layer_active(slint::ModelRc::from(model));
                    
                    vec![]
                },
                "Screen" => ss.displays.iter().map(|d| slint::SharedString::from(d.name.as_str())).collect(),
                "Window" => ss.windows.iter().map(|w| slint::SharedString::from(w.name.as_str())).collect(),
                "Camera" => ss.cameras.iter().map(|(_, n)| slint::SharedString::from(n.as_str())).collect(),
                "NDI" => ss.ndi_sources.iter().map(|n| slint::SharedString::from(n.as_str())).collect(),
                 "Presentation" => {
                    let p_wins: Vec<_> = ss.windows.iter().filter(|w| w.is_presentation).collect();
                    if p_wins.is_empty() {
                         vec![slint::SharedString::from("Waiting for Slide Show...")]
                    } else {
                         p_wins.iter().map(|w| slint::SharedString::from(format!("READY: {}", w.name).as_str())).collect()
                    }
                },
                _ => vec![],
            };
            let model = std::rc::Rc::new(slint::VecModel::from(names));
            ui.set_device_list(slint::ModelRc::from(model));
        }
    });

    // 3. Select Device (Start Capture)
    let ui_handle_device = ui.as_weak();
    let ss_device = source_state.clone();
    let res_device = res_state.clone();
    ui.on_select_device(move |device_idx| {
        let device_idx = device_idx as usize;
        let mut ss = match ss_device.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner()
        };
        
        let layer_idx = ss.selected_layer_idx;
        // Access layer
        // We need to mutate layer, so lets get mutable ref
        // But we also need to access ss.displays which is in ss.
        // So we can't borrow ss.layers[layer_idx] independently easily if we also use ss.
        // Rust borrow checker...
        
        let current_type = ss.layers[layer_idx].current_type.clone();
        let tx = ss.layers[layer_idx].tx.as_ref().unwrap().clone();
        let res_clone = res_device.clone();

        match current_type.as_str() {
            "Screen" => {
                if device_idx < ss.displays.len() {
                    // Stop previous
                    if let Some(ref stop) = ss.layers[layer_idx].active_stop { stop.stop(); }
                    
                    let (id, name, _width, _height) = {
                        let d = &ss.displays[device_idx];
                        (d.id, d.name.clone(), d.width, d.height)
                    };
                    
                    let stop = capture::capture_display(id, tx, res_clone);
                    
                    if let Some(ui) = ui_handle_device.upgrade() {
                         ui.set_source_name(slint::SharedString::from(name.as_str()));
                    }
                    ss.layers[layer_idx].active_stop = Some(stop);
                    ss.layers[layer_idx].current_source_name = name;
                }
            }
            "Window" => {
                if device_idx < ss.windows.len() {
                     if let Some(ref stop) = ss.layers[layer_idx].active_stop { stop.stop(); }
                     let (id, name) = {
                         let w = &ss.windows[device_idx];
                         (w.id, w.name.clone())
                     };
                     println!("[UI] Layer {} -> Window: {}", layer_idx, name);
                     let stop = capture::capture_window(id, tx, res_clone);
                     if let Some(ui) = ui_handle_device.upgrade() {
                         ui.set_source_name(slint::SharedString::from(name.as_str()));
                     }
                     ss.layers[layer_idx].active_stop = Some(stop);
                     ss.layers[layer_idx].current_source_name = name;
                }
            }
            "Camera" => {
                if device_idx < ss.cameras.len() {
                     let (cam_idx, name) = {
                        let (idx, ref n) = ss.cameras[device_idx];
                        (idx, n.clone())
                     };
                     let stop = capture::camera::start_camera(cam_idx, tx, res_clone);
                     if let Some(ui) = ui_handle_device.upgrade() {
                         ui.set_source_name(slint::SharedString::from(name.as_str()));
                     }
                     ss.layers[layer_idx].active_stop = Some(stop);
                     ss.layers[layer_idx].current_source_name = name;
                }
            }
            "NDI" => {
                if device_idx < ss.ndi_sources.len() {
                     if let Some(ref stop) = ss.layers[layer_idx].active_stop { stop.stop(); }
                     let name = ss.ndi_sources[device_idx].clone();
                     let stop = ndi::receiver::start_ndi_capture(name.clone(), tx, res_clone);
                     if let Some(ui) = ui_handle_device.upgrade() {
                         ui.set_source_name(slint::SharedString::from(name.as_str()));
                     }
                     ss.layers[layer_idx].active_stop = Some(stop);
                     ss.layers[layer_idx].current_source_name = name;
                }
            }
            "Presentation" => {
                let pres_windows: Vec<_> = ss.windows.iter().filter(|w| w.is_presentation).cloned().collect();
                if device_idx < pres_windows.len() {
                    if let Some(ref stop) = ss.layers[layer_idx].active_stop { stop.stop(); }
                    let w = &pres_windows[device_idx];
                     println!("[UI] Layer {} -> Presentation: {}", layer_idx, w.name);
                    let stop = capture::capture_window(w.id, tx, res_clone);
                    if let Some(ui) = ui_handle_device.upgrade() {
                         ui.set_source_name(slint::SharedString::from(w.name.as_str()));
                    }
                    ss.layers[layer_idx].active_stop = Some(stop);
                    ss.layers[layer_idx].current_source_name = w.name.clone();
                }
            }
            _ => {}
        }
        
        // Update Layer Active Status Globally (Refresh all)
        if let Some(ui) = ui_handle_device.upgrade() {
            let mut flags = Vec::new();
            for l in &ss.layers { flags.push(l.active_stop.is_some()); }
            let model = std::rc::Rc::new(slint::VecModel::from(flags));
            ui.set_layer_active(slint::ModelRc::from(model));
        }
    });

    // 4. Refresh Sources
    let ui_handle_refresh = ui.as_weak();
    let ss_refresh = source_state.clone();
    ui.on_refresh_sources(move || {
        let ss_clone = ss_refresh.clone();
        let ui_weak = ui_handle_refresh.clone();
        
        tokio::spawn(async move {
            let cameras = capture::camera::list_cameras();
            let (displays, windows) = capture::list_sources();
            let ndi = ndi::receiver::discover_sources().await;

            let mut ss = match ss_clone.lock() {
                Ok(g) => g,
                Err(p) => p.into_inner()
            };
            ss.cameras = cameras;
            ss.displays = displays;
            ss.windows = windows;
            ss.ndi_sources = ndi;
            
            // Re-populate list for CURRENT layer
            let idx = ss.selected_layer_idx;
            let current_type = ss.layers[idx].current_type.clone();
            
            // ... (Logic same as above, copy-paste or abstract? Copy for now)
            let names: Vec<slint::SharedString> = match current_type.as_str() {
                "Screen" => ss.displays.iter().map(|d| slint::SharedString::from(d.name.as_str())).collect(),
                "Window" => ss.windows.iter().map(|w| slint::SharedString::from(w.name.as_str())).collect(),
                "Camera" => ss.cameras.iter().map(|(_, n)| slint::SharedString::from(n.as_str())).collect(),
                "NDI" => ss.ndi_sources.iter().map(|n| slint::SharedString::from(n.as_str())).collect(),
                "Presentation" => {
                     let p_wins: Vec<_> = ss.windows.iter().filter(|w| w.is_presentation).collect();
                     if p_wins.is_empty() { vec![slint::SharedString::from("Waiting...")] } 
                     else { p_wins.iter().map(|w| slint::SharedString::from(w.name.as_str())).collect() }
                },
                _ => vec![],
            };
            
             let _ = slint::invoke_from_event_loop(move || {
                let model = std::rc::Rc::new(slint::VecModel::from(names));
                if let Some(ui) = ui_weak.upgrade() {
                    ui.set_device_list(slint::ModelRc::from(model));
                }
            });
        });
    });

    // 5. Toggle Output
    let output_state_ui = output_state.clone();
    let ui_handle_toggle = ui.as_weak();
    let ss_toggle = source_state.clone();
    let res_toggle = res_state.clone();
    let key_state_toggle = key_state.clone();
    ui.on_toggle_output(move |output_type, state| {
         if output_type == "NDI" { output_state_ui.set_ndi(state); }
         else if output_type == "DeckLink" { output_state_ui.set_decklink(state); }
         else if output_type == "HybridKey" { key_state_toggle.set_enabled(state); }
         else if output_type == "VirtualScreen" {
             // Virtual Screen logic...
             let mut ss = match ss_toggle.lock() { Ok(g)=>g, Err(p)=>p.into_inner() };
             if state {
                 let (out_w, out_h) = res_toggle.output();
                 let (w, h) = if out_w > 0 { (out_w, out_h) } else { (1920,1080) };
                 match capture::VirtualDisplay::new(w, h, "Lumina Virtual") {
                     Some(vd) => {
                         // Auto-set Layer 0 to Virtual Screen?
                         // Or just auto-capture on Layer 0?
                         // Let's force Layer 0 to be Virtual Screen
                         let layer_idx = 0;
                         if let Some(ref stop) = ss.layers[layer_idx].active_stop { stop.stop(); }
                         
                         let tx = ss.layers[layer_idx].tx.as_ref().unwrap().clone();
                         let stop = capture::capture_display(vd.display_id(), tx, res_toggle.clone());
                         
                         ss.layers[layer_idx].active_stop = Some(stop);
                         ss.layers[layer_idx].current_type = "Screen".to_string();
                         // Also update UI if Layer 0 selected
                         if ss.selected_layer_idx == 0 {
                             // Need to trigger UI update? 
                             // Slint properties need explicit set.
                             if let Some(ui) = ui_handle_toggle.upgrade() {
                                 ui.set_selected_source_type("Screen".into());
                                 // Device list update is hard here without recalling refresh.
                             }
                         }
                         ss.virtual_display = Some(vd);
                         println!("[VirtualScreen] Active on Layer 0");
                     }
                     None => eprintln!("Failed to create VirtualDisplay"),
                 }
             } else {
                 // Stop Layer 0? only if it's the virtual display?
                 // For safety just stop Layer 0 if virtual_display is present
                 if ss.virtual_display.is_some() {
                      if let Some(ref stop) = ss.layers[0].active_stop { stop.stop(); }
                      ss.layers[0].active_stop = None;
                      ss.virtual_display = None;
                      println!("[VirtualScreen] Stopped");
                 }
             }
         } else if output_type == "DeckLinkMonitor" {
             // Monitor logic (same as before)
             let mut ss = match ss_toggle.lock() { Ok(g)=>g, Err(p)=>p.into_inner() };
              if state {
                use crate::output::decklink::{DeckLinkManager};
                let devices = DeckLinkManager::enumerate_devices();
                if let Some(dev) = devices.into_iter().next() {
                    match output::decklink_input::DeckLinkInput::start(&dev) {
                        Some((input, rx)) => {
                            ss.decklink_input = Some(input);
                            ss.monitor_rx = Some(rx);
                        }
                        None => {}
                    }
                }
             } else {
                 ss.decklink_input = None;
                 ss.monitor_rx = None;
             }
         }
    });
    
    // Key Param Params
    let key_state_param = key_state.clone();
    ui.on_set_key_param(move |param, value| {
        let v = value as u32;
         match param.as_str() {
            "tolerance" => key_state_param.set_tolerance(v),
            "softness" => key_state_param.set_softness(v),
            "luma-low" => key_state_param.set_luma_low(v),
            "color-green" => key_state_param.set_key_color(0x00, 0xB1, 0x40),
            "color-blue" => key_state_param.set_key_color(0x00, 0x33, 0xCC),
            "luma-softness" => key_state_param.set_luma_softness(v as u32),
            "spill" => key_state_param.set_spill_suppress(v > 0),
            "smoothing" => key_state_param.set_smoothing(v),
            _ => {},
        }
    });

    // Auto-Start BG (Layer 0)
    {
        let mut ss = source_state.lock().unwrap();
        if !ss.displays.is_empty() {
            let (id, name) = {
                let d = &ss.displays[0];
                (d.id, d.name.clone())
            };
            let tx = ss.layers[0].tx.as_ref().unwrap().clone();
            let stop = capture::capture_display(id, tx, res_state.clone());
            ss.layers[0].active_stop = Some(stop);
            ss.layers[0].current_source_name = name.clone();
            println!("[Main] Auto-started Layer 0: {}", name);
            
            // Update UI for Layer 0
            ui.set_source_name(slint::SharedString::from(name.as_str()));
        }
    }

    // Auto-Latch Logic (Scans specifically for "Presentation" type on ANY layer that is configured for it but not started)
    let ss_latch = source_state.clone();
    let res_latch = res_state.clone();
    let ui_latch = ui.as_weak();
    
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_millis(3000)).await; // Optimized to 3s to reduce CPU load
            
            // Check all layers
            // To avoid holding lock too long, we gather needs first
            let mut needs_latch = Vec::new(); // (layer_idx, Sender)
            
            {
                let ss = match ss_latch.lock() { Ok(g)=>g, Err(p)=>p.into_inner() };
                for (i, layer) in ss.layers.iter().enumerate() {
                    if layer.current_type == "Presentation" && layer.active_stop.is_none() {
                        if let Some(tx) = &layer.tx {
                            needs_latch.push((i, tx.clone()));
                        }
                    }
                }
            }
            
            if !needs_latch.is_empty() {
                // Heavy task: List sources (Blocking I/O)
                let sources_result = tokio::task::spawn_blocking(move || {
                    capture::list_sources()
                }).await;

                if let Ok((_, windows)) = sources_result {
                    if let Some(target) = windows.iter().find(|w| w.is_presentation) {
                        let mut ss = match ss_latch.lock() { Ok(g)=>g, Err(p)=>p.into_inner() };
                        
                        for (idx, tx) in needs_latch {
                            // Double check still needed?
                            if ss.layers[idx].current_type == "Presentation" && ss.layers[idx].active_stop.is_none() {
                                 println!("[AutoLatch] Layer {} -> PPT: {}", idx, target.name);
                                 let stop = capture::capture_window(target.id, tx, res_latch.clone());
                                 ss.layers[idx].active_stop = Some(stop);
                             ss.layers[idx].current_source_name = target.name.clone();
                             
                             // UI Update if this layer selected?
                             if ss.selected_layer_idx == idx {
                                  if let Some(ui) = ui_latch.upgrade() {
                                      ui.set_source_name(slint::SharedString::from(target.name.as_str()));
                                  }
                             }
                        }
                    }
                }
            }
        }
        }
    });

    // Preview Timer ... (Same as before)
    let ui_handle_preview = ui.as_weak();
    let preview_frame_ui = preview_frame.clone();
    let timer = slint::Timer::default();
    timer.start(
        slint::TimerMode::Repeated,
        std::time::Duration::from_millis(16),
        move || {
            let frame_data = if let Ok(mut guard) = preview_frame_ui.try_lock() {
                guard.take()
            } else { None };

            if let Some(frame) = frame_data {
                if let Some(ui) = ui_handle_preview.upgrade() {
                    let w = frame.width;
                    let h = frame.height;
                    if let Some(data) = frame.data {
                         let mut pixel_buffer = slint::SharedPixelBuffer::<slint::Rgba8Pixel>::new(w, h);
                         let expected_len = (w * h * 4) as usize;
                         let slice = pixel_buffer.make_mut_bytes();
                         if data.len() >= expected_len { slice.copy_from_slice(&data[..expected_len]); }
                         else { let l = std::cmp::min(data.len(), slice.len()); slice[..l].copy_from_slice(&data[..l]); }
                         ui.set_preview_image(slint::Image::from_rgba8(pixel_buffer));
                    }
                }
            }
        },
    );

    ui.on_toggled_layer_key(move |idx, enabled| {
        let idx = idx as usize;
        if idx < layer_key_enables_ui.len() {
            layer_key_enables_ui[idx].store(enabled, Ordering::Relaxed);
            println!("[Main] Layer {} Key Enable: {}", idx, enabled);
        }
    });

    let ui_refresh = ui.as_weak();
    let ss_refresh = source_state.clone();
    ui.on_refresh_sources(move || {
        println!("[Main] Refreshing sources...");
        let (displays, windows) = capture::list_sources();
        let cameras = capture::camera::list_cameras();
        
        // Update SourceState
        let mut ss = ss_refresh.lock().unwrap();
        ss.displays = displays;
        ss.windows = windows;
        ss.cameras = cameras; // And NDI too if we had logic
        
        // Update UI Device List based on current type of selected layer
        let current_layer_idx = ss.selected_layer_idx;
        let current_type = ss.layers[current_layer_idx].current_type.clone();
        
        let mut new_list = Vec::new();
        if current_type == "Screen" {
             for d in &ss.displays { new_list.push(slint::SharedString::from(d.name.as_str())); }
        } else if current_type == "Window" || current_type == "Presentation" {
             for w in &ss.windows { new_list.push(slint::SharedString::from(w.name.as_str())); }
        } else if current_type == "Camera" {
             for (_, name) in &ss.cameras { new_list.push(slint::SharedString::from(name.as_str())); }
        }
        
        if let Some(ui) = ui_refresh.upgrade() {
             let model = std::rc::Rc::new(slint::VecModel::from(new_list));
             ui.set_device_list(slint::ModelRc::from(model));
        }
    });

    ui.run()
}
