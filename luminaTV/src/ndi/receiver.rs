use crate::ndi::ffi;
use tokio::time::Duration;
use std::{ffi::CStr, ptr};
use tokio::sync::mpsc;
use crate::core::VideoFrame;

pub async fn init_ndi_receiver(_tx: mpsc::Sender<VideoFrame>) {
    println!("Initializing NDI receiver...");

    unsafe {
        if !ffi::NDIlib_initialize() {
            eprintln!("Failed to initialize NDI library.");
            return;
        }
    }
    println!("NDI library initialized.");

    let p_ndi_find = unsafe {
        ffi::NDIlib_find_create_v2(ptr::null_mut()) // Pass null to use default settings
    };
    if p_ndi_find.is_null() {
        eprintln!("Failed to create NDI finder.");
        unsafe { ffi::NDIlib_destroy(); }
        return;
    }
    println!("NDI finder created.");

    println!("Searching for NDI sources...");

    // Use synchronous sleep for discovery search to avoid holding non-Send pointer across await
    std::thread::sleep(Duration::from_secs(5));

    let p_sources = unsafe { ffi::NDIlib_find_get_current_sources(p_ndi_find, ptr::null_mut()) };
    if p_sources.is_null() {
        eprintln!("Failed to get NDI sources.");
        unsafe {
            ffi::NDIlib_find_destroy(p_ndi_find);
            ffi::NDIlib_destroy();
        }
        return;
    }

    let sources_vec = unsafe {
        let mut vec = Vec::new();
        let mut i = 0;
        let mut p_source_info = *p_sources.add(i);
        while !p_source_info.p_ndi_name.is_null() {
            vec.push(p_source_info);
            i += 1;
            p_source_info = *p_sources.add(i);
        }
        vec
    };

    if sources_vec.is_empty() {
        println!("No NDI sources found.");
    } else {
        println!("Discovered NDI sources:");
        for (i, source_info) in sources_vec.iter().enumerate() {
            let ndi_name_cstr = unsafe { CStr::from_ptr(source_info.p_ndi_name) };
            let ndi_name = ndi_name_cstr.to_string_lossy();
            println!("  {}: {}", i, ndi_name);
        }
    }

    println!("NDI receiver initialized.");

    unsafe {
        ffi::NDIlib_find_destroy(p_ndi_find);
        ffi::NDIlib_destroy();
    }
}
