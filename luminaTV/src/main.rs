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
use tokio::sync::Mutex;
use crate::core::{StopSignal, ResolutionState, RESOLUTION_PRESETS};

slint::include_modules!();

struct SourceState {
    current_type: String,
    cameras: Vec<(usize, String)>,
    displays: Vec<capture::mac::DisplayInfo>,
    windows: Vec<capture::mac::WindowInfo>,
    ndi_sources: Vec<String>,
    active_stop: Option<StopSignal>,
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

    let ui = AppWindow::new()?;

    // Enumerate sources
    println!("[Main] Enumerating cameras...");
    let cameras = capture::camera::list_cameras();
    
    println!("[Main] Enumerating sources (Displays & Windows)...");
    let (displays, windows) = capture::list_sources();

    println!("[Main] Startup: Found {} cameras, {} displays, {} windows", 
        cameras.len(), displays.len(), windows.len());




    // Initial device list (Screen)
    let display_names: Vec<slint::SharedString> = displays.iter()
        .map(|d| slint::SharedString::from(d.name.as_str()))
        .collect();
    let display_model = std::rc::Rc::new(slint::VecModel::from(display_names));
    ui.set_device_list(slint::ModelRc::from(display_model.clone()));

    let source_state = Arc::new(std::sync::Mutex::new(SourceState {
        current_type: "Screen".to_string(),
        cameras,
        displays,
        windows,
        ndi_sources: Vec::new(),
        active_stop: None,
        virtual_display: None,
        decklink_input: None,
        monitor_rx: None,
    }));

    // Frame channel — buffer for dual-output stability (NDI + DeckLink)
    let (tx, rx) = tokio::sync::mpsc::channel(4);

    // Start compositor
    let output_state_comp = output_state.clone();
    let preview_frame_comp = preview_frame.clone();
    let res_comp = res_state.clone();
    tokio::spawn(async move {
        core::compositor::run_compositor(rx, output_state_comp, preview_frame_comp, res_comp).await;
    });

    // NDI receiver background - (Removed, using on-demand discovery)
    // tokio::spawn(async move {
    //    // ndi::receiver::init_ndi_receiver(tx_ndi).await;
    // });

    // UltraStudio / DeckLink background
    tokio::spawn(async move {
        println!("[Main] Enumerating DeckLink devices...");
        let devices = output::decklink::DeckLinkManager::enumerate_devices();
        println!("[Main] Found {} DeckLink devices.", devices.len());
        for d in devices {
             println!("[Main] - {} (Index: {})", d.name, d.index);
        }
        // output::ultrastudio::init_ultrastudio_output();
    });


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

    let ui_handle = ui.as_weak();
    let ss_type = source_state.clone();
    ui.on_select_source_type(move |source_type| {
        let st = source_type.to_string();
        println!("[UI] Source type: {}", st);

        let mut ss = match ss_type.lock() {
            Ok(g) => g,
            Err(poisoned) => { eprintln!("[UI] Mutex poisoned, recovering"); poisoned.into_inner() }
        };
        if let Some(ref stop) = ss.active_stop { stop.stop(); }
        ss.active_stop = None;
        ss.current_type = st.clone();

        if let Some(ui) = ui_handle.upgrade() {
            let names: Vec<slint::SharedString> = match st.as_str() {
                "Screen" => ss.displays.iter().map(|d| slint::SharedString::from(d.name.as_str())).collect(),
                "Window" => ss.windows.iter().map(|w| slint::SharedString::from(w.name.as_str())).collect(),
                "Camera" => ss.cameras.iter().map(|(_, n)| slint::SharedString::from(n.as_str())).collect(),
                "NDI" => ss.ndi_sources.iter().map(|n| slint::SharedString::from(n.as_str())).collect(),
                _ => vec![],
            };
            let model = std::rc::Rc::new(slint::VecModel::from(names));
            ui.set_device_list(slint::ModelRc::from(model));
            ui.set_source_name(slint::SharedString::from(format!("{} (select device)", st)));
            ui.set_source_details(slint::SharedString::from(""));
        }
    });

    let ui_handle2 = ui.as_weak();
    let ss_device = source_state.clone();
    let tx_device = tx.clone();
    let res_device = res_state.clone();
    ui.on_select_device(move |idx| {
        let idx = idx as usize;
        let mut ss = match ss_device.lock() {
            Ok(g) => g,
            Err(poisoned) => { eprintln!("[UI] Mutex poisoned, recovering"); poisoned.into_inner() }
        };
        if let Some(ref stop) = ss.active_stop { stop.stop(); }
        ss.active_stop = None;

        let tx_clone = tx_device.clone();
        let res_clone = res_device.clone();

        match ss.current_type.as_str() {
            "Screen" => {
                if idx < ss.displays.len() {
                    let d = &ss.displays[idx];
                    let stop = capture::capture_display(d.id, tx_clone, res_clone);
                    if let Some(ui) = ui_handle2.upgrade() {
                        ui.set_source_name(slint::SharedString::from(d.name.as_str()));
                        ui.set_source_details(slint::SharedString::from(
                            format!("{}x{} | ~60fps", d.width, d.height)
                        ));
                    }
                    ss.active_stop = Some(stop);
                }
            }
            "Window" => {
                if idx < ss.windows.len() {
                    let w = &ss.windows[idx];
                    println!("[UI] Selecting window: {} (ID {})", w.name, w.id);
                    let stop = capture::capture_window(w.id, tx_clone, res_clone);
                    if let Some(ui) = ui_handle2.upgrade() {
                        ui.set_source_name(slint::SharedString::from(w.name.as_str()));
                        ui.set_source_details(slint::SharedString::from(
                            format!("{}x{} | ~60fps", w.width, w.height)
                        ));
                    }
                    ss.active_stop = Some(stop);
                }
            }
            "Camera" => {
                if idx < ss.cameras.len() {
                    let (cam_idx, ref name) = ss.cameras[idx];
                    let name_str = name.clone();
                    let stop = capture::camera::start_camera(cam_idx, tx_clone, res_clone);
                    if let Some(ui) = ui_handle2.upgrade() {
                        ui.set_source_name(slint::SharedString::from(name_str.as_str()));
                        ui.set_source_details(slint::SharedString::from("Connecting..."));
                    }
                    ss.active_stop = Some(stop);
                }
            }
            "NDI" => {
                if idx < ss.ndi_sources.len() {
                    let name = ss.ndi_sources[idx].clone();
                    let stop = ndi::receiver::start_ndi_capture(name.clone(), tx_clone, res_clone);
                     if let Some(ui) = ui_handle2.upgrade() {
                        ui.set_source_name(slint::SharedString::from(name.as_str()));
                        ui.set_source_details(slint::SharedString::from("NDI Connecting..."));
                    }
                    ss.active_stop = Some(stop);
                }
            }
            "Presentation" => {
                // Find presentation windows
                let pres_windows: Vec<_> = ss.windows.iter().filter(|w| w.is_presentation).collect();
                if idx < pres_windows.len() {
                    let w = pres_windows[idx];
                    println!("[UI] Starting Presentation Capture: {}", w.name);
                    let stop = capture::capture_window(w.id, tx_clone, res_clone);
                    if let Some(ui) = ui_handle2.upgrade() {
                        ui.set_source_name(slint::SharedString::from(w.name.as_str()));
                        ui.set_source_details(slint::SharedString::from("Presentation (Window)"));
                    }
                    ss.active_stop = Some(stop);
                } else {
                    println!("[UI] Presentation selected but index out of bounds (or waiting)");
                }
            }
            _ => {}
        }
    });

    let ui_handle3 = ui.as_weak();
    let ss_refresh = source_state.clone();
    ui.on_refresh_sources(move || {
        let ss_clone = ss_refresh.clone();
        let ui_weak = ui_handle3.clone();
        
        tokio::spawn(async move {
            println!("[UI] Refreshing sources...");
            let cameras = capture::camera::list_cameras();
            let (displays, windows) = capture::list_sources();
            let ndi = ndi::receiver::discover_sources().await;

            let mut ss = match ss_clone.lock() {
                Ok(g) => g,
                Err(poisoned) => { eprintln!("[UI] Mutex poisoned, recovering"); poisoned.into_inner() }
            };
            ss.cameras = cameras;
            ss.displays = displays;
            ss.windows = windows;
            ss.ndi_sources = ndi;

            println!("[UI] Refreshed: {} cameras, {} displays, {} windows, {} NDI",
                ss.cameras.len(), ss.displays.len(), ss.windows.len(), ss.ndi_sources.len());

            let current_type = ss.current_type.clone();
            
            // Re-populate UI list based on current selection
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
            
            // Update UI on main thread logic (via Slint's thread-safe upgrade)
            // Slint handles are thread-safe for invoking methods? 
            // set_device_list is a property setter, usually needs run_on_ui_thread or similar if from background.
            // But here we are upgrading a Weak handle. 
            // IMPORTANT: Slint's generated code `set_device_list` executes on the calling thread?
            // If calling thread is background, it might panic or be UB if Slint backend isn't thread-safe (Winit isn't).
            // Checked Slint docs: `invoke_from_event_loop` is needed. 
            // `WeakAppWindow` is Send. `upgrade()` returns `AppWindow` which is Send?
            // Actually, `active_stop` logic above runs in callback (Main Thread).
            // But this spawn is background.
            // I need to use `slint::invoke_from_event_loop`.
            
            // Move names (Vec<SharedString>) into the closure, construct model on UI thread
            let _ = slint::invoke_from_event_loop(move || {
                let model = std::rc::Rc::new(slint::VecModel::from(names));
                let model_rc = slint::ModelRc::from(model);
                if let Some(ui) = ui_weak.upgrade() {
                    ui.set_device_list(model_rc);
                }
            });
        });
    });

    let output_state_ui = output_state.clone();
    let ui_handle4 = ui.as_weak();
    let ss_toggle = source_state.clone();
    let tx_toggle = tx.clone();
    let res_toggle = res_state.clone();
    ui.on_toggle_output(move |output_type, state| {
        println!("[UI] Toggle {} output: {}", output_type, state);
        if output_type == "NDI" { output_state_ui.set_ndi(state); }
        else if output_type == "DeckLink" { output_state_ui.set_decklink(state); }
        else if output_type == "VirtualScreen" {
            let mut ss = match ss_toggle.lock() {
                Ok(g) => g,
                Err(p) => { eprintln!("[UI] Mutex poisoned, recovering"); p.into_inner() }
            };

            if state {
                // Create virtual display at output resolution
                let (out_w, out_h) = res_toggle.output();
                let (w, h) = if out_w > 0 && out_h > 0 { (out_w, out_h) } else { (1920, 1080) };

                match capture::VirtualDisplay::new(w, h, "LuminaTV Virtual") {
                    Some(vd) => {
                        let display_id = vd.display_id();
                        println!("[VirtualScreen] ON — Display ID: {}", display_id);

                        // Stop any existing capture
                        if let Some(ref stop) = ss.active_stop {
                            stop.stop();
                        }

                        // Auto-start capturing the virtual display
                        let stop = capture::capture_display(
                            display_id, tx_toggle.clone(), res_toggle.clone()
                        );
                        ss.active_stop = Some(stop);
                        ss.current_type = "Screen".to_string();
                        ss.virtual_display = Some(vd);
                    }
                    None => {
                        eprintln!("[VirtualScreen] Failed to create (macOS 14+ required)");
                    }
                }
            } else {
                // Stop capture and destroy virtual display
                if let Some(ref stop) = ss.active_stop {
                    stop.stop();
                }
                ss.active_stop = None;
                ss.virtual_display = None; // Drop triggers destroy
                println!("[VirtualScreen] OFF — Virtual display removed");
            }
        }
        else if output_type == "DeckLinkMonitor" {
            let mut ss = match ss_toggle.lock() {
                Ok(g) => g,
                Err(p) => { eprintln!("[UI] Mutex poisoned, recovering"); p.into_inner() }
            };

            if state {
                // Find and start DeckLink input capture
                use crate::output::decklink::{DeckLinkManager};
                let devices = DeckLinkManager::enumerate_devices();
                if let Some(dev) = devices.into_iter().next() {
                    match output::decklink_input::DeckLinkInput::start(&dev) {
                        Some((input, rx)) => {
                            println!("[DeckLink Monitor] ON — Monitoring started");
                            ss.decklink_input = Some(input);
                            ss.monitor_rx = Some(rx);
                        }
                        None => {
                            eprintln!("[DeckLink Monitor] Failed to start input capture");
                        }
                    }
                } else {
                    eprintln!("[DeckLink Monitor] No DeckLink device found");
                }
            } else {
                // Stop DeckLink input capture
                ss.decklink_input = None; // Drop stops streams
                ss.monitor_rx = None;
                println!("[DeckLink Monitor] OFF");
            }
        }

        if let Some(ui) = ui_handle4.upgrade() {
            let ndi_s = if output_state_ui.is_ndi_enabled() { "ON" } else { "OFF" };
            let dl_s = if output_state_ui.is_decklink_enabled() { "ON" } else { "OFF" };
            let (vs_s, mon_s) = {
                let ss = match ss_toggle.lock() {
                    Ok(g) => g,
                    Err(p) => p.into_inner()
                };
                (
                    if ss.virtual_display.is_some() { "ON" } else { "OFF" },
                    if ss.decklink_input.is_some() { "ON" } else { "OFF" },
                )
            };
            ui.set_source_details(slint::SharedString::from(
                format!("NDI:{} DL:{} VS:{} Mon:{}", ndi_s, dl_s, vs_s, mon_s)
            ));
        }
    });

    // Auto-start main display capture
    {
        let ss = match source_state.lock() {
            Ok(g) => g,
            Err(poisoned) => { eprintln!("[UI] Mutex poisoned, recovering"); poisoned.into_inner() }
        };
        if !ss.displays.is_empty() {
            let d = &ss.displays[0];
            let stop = capture::capture_display(d.id, tx.clone(), res_state.clone());
            drop(ss);
            match source_state.lock() {
                Ok(mut g) => g.active_stop = Some(stop),
                Err(poisoned) => { eprintln!("[UI] Mutex poisoned, recovering"); poisoned.into_inner().active_stop = Some(stop); }
            }
        }
    }

    // === Auto-Latch / Presentation Scanner Loop ===
    let ss_latch = source_state.clone();
    let ui_latch = ui.as_weak();
    let tx_latch = tx.clone();
    let res_latch = res_state.clone();
    
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_millis(1000)).await;

            let (is_pres_mode, needs_start) = {
                let ss = match ss_latch.lock() {
                    Ok(g) => g,
                    Err(p) => p.into_inner(),
                };
                (ss.current_type == "Presentation", ss.active_stop.is_none())
            };

            if is_pres_mode && needs_start {
                // Scan quickly for presentation
                let (_, windows) = capture::list_sources();
                if let Some(target) = windows.iter().find(|w| w.is_presentation) {
                     println!("[AutoLatch] Found Slide Show: {}", target.name);
                     
                     // Start capture
                     let stop = capture::capture_window(target.id, tx_latch.clone(), res_latch.clone());
                     
                     // Update State
                     {
                        let mut ss = match ss_latch.lock() {
                            Ok(g) => g,
                            Err(p) => p.into_inner(),
                        };
                        ss.active_stop = Some(stop);
                     }

                     // Update UI
                     let ui_weak = ui_latch.clone();
                     let name = target.name.clone();
                     let _ = slint::invoke_from_event_loop(move || {
                        if let Some(ui) = ui_weak.upgrade() {
                             ui.set_source_name(slint::SharedString::from(name.as_str()));
                             ui.set_source_details(slint::SharedString::from("Auto-Latched Presentation"));
                        }
                     });
                }
            }
        }
    });

    // Preview timer (~60fps)
    let ui_handle_preview = ui.as_weak();
    let preview_frame_ui = preview_frame.clone();
    let timer = slint::Timer::default();
    timer.start(
        slint::TimerMode::Repeated,
        std::time::Duration::from_millis(16),
        move || {
            let frame_data = if let Ok(mut guard) = preview_frame_ui.try_lock() {
                guard.take()
            } else {
                None
            };

            if let Some(frame) = frame_data {
                if let Some(ui) = ui_handle_preview.upgrade() {
                    let w = frame.width;
                    let h = frame.height;
                    if let Some(data) = frame.data {
                        let mut pixel_buffer = slint::SharedPixelBuffer::<slint::Rgba8Pixel>::new(w, h);
                        let expected_len = (w * h * 4) as usize;
                        let slice = pixel_buffer.make_mut_bytes();

                        if data.len() >= expected_len {
                            slice.copy_from_slice(&data[..expected_len]);
                        } else {
                            let copy_len = std::cmp::min(data.len(), slice.len());
                            slice[..copy_len].copy_from_slice(&data[..copy_len]);
                        }

                        let image = slint::Image::from_rgba8(pixel_buffer);
                        ui.set_preview_image(image);
                    }
                }
            }
        },
    );

    ui.run()
}
