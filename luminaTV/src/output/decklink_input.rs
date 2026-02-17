/// DeckLink Input Capture — Monitors SDI input independently from output path.
///
/// Used for preview/monitoring when DeckLink hardware keying is active:
///   SDI IN → DeckLink card → SDI OUT   (hardware keying, untouched)
///   SDI IN → This module → VideoFrame → Compositor → Preview   (monitoring)
///
/// Based on DeckLink SDK 15.3 — IDeckLinkInput vtable.

use std::ffi::c_void;
use std::ptr;
use std::sync::Arc;
use std::time::Instant;
use crate::output::decklink::DeckLinkDevice;
use crate::output::decklink_sys::HRESULT;
use crate::core::VideoFrame;

const S_OK: HRESULT = 0;
const BMD_FORMAT_8BIT_BGRA: u32 = 0x42475241; // 'BGRA'
const BMD_MODE_HD1080P60: u32 = 0x48703630;   // 'Hp60'
const BMD_VIDEO_INPUT_FLAG_DEFAULT: u32 = 0;

// IID_IDeckLinkInput = {40,95,DB,82-E2,94-4B,8C-AA,A8-3B,9E,80,C4,93,36}
const IID_IDECK_LINK_INPUT: [u8; 16] = [
    0x40, 0x95, 0xDB, 0x82, 0xE2, 0x94, 0x4B, 0x8C,
    0xAA, 0xA8, 0x3B, 0x9E, 0x80, 0xC4, 0x93, 0x36,
];

// IID_IDeckLinkVideoBuffer (same as in decklink.rs)
const IID_VIDEO_BUFFER: [u8; 16] = [
    0xCC, 0xB4, 0xB6, 0x4A, 0x5C, 0x86, 0x4E, 0x02,
    0xB7, 0x78, 0x88, 0x5D, 0x35, 0x27, 0x09, 0xFE,
];

// ======== IUnknown vtable — used to QI on any COM object ========
#[repr(C)]
struct IUnknownVtbl {
    pub query_interface: unsafe extern "C" fn(*mut c_void, *const u8, *mut *mut c_void) -> HRESULT,
    pub add_ref: unsafe extern "C" fn(*mut c_void) -> u32,
    pub release: unsafe extern "C" fn(*mut c_void) -> u32,
}

// ======== IDeckLinkInput vtable ========
#[repr(C)]
struct IDeckLinkInputVtbl {
    // IUnknown
    pub query_interface: unsafe extern "C" fn(*mut c_void, *const u8, *mut *mut c_void) -> HRESULT,
    pub add_ref: unsafe extern "C" fn(*mut c_void) -> u32,
    pub release: unsafe extern "C" fn(*mut c_void) -> u32,
    // IDeckLinkInput
    pub does_support_video_mode: *const c_void,
    pub get_display_mode: *const c_void,
    pub get_display_mode_iterator: *const c_void,
    pub set_screen_preview_callback: *const c_void,
    pub enable_video_input: unsafe extern "C" fn(*mut c_void, u32, u32, u32) -> HRESULT,
    pub enable_video_input_with_allocator: *const c_void,
    pub disable_video_input: unsafe extern "C" fn(*mut c_void) -> HRESULT,
    pub get_available_video_frame_count: *const c_void,
    pub enable_audio_input: *const c_void,
    pub disable_audio_input: *const c_void,
    pub get_available_audio_sample_frame_count: *const c_void,
    pub start_streams: unsafe extern "C" fn(*mut c_void) -> HRESULT,
    pub stop_streams: unsafe extern "C" fn(*mut c_void) -> HRESULT,
    pub pause_streams: *const c_void,
    pub flush_streams: *const c_void,
    pub set_callback: unsafe extern "C" fn(*mut c_void, *mut c_void) -> HRESULT,
}

#[repr(C)]
struct IDeckLinkInput {
    vtable: *const IDeckLinkInputVtbl,
}

// ======== IDeckLinkVideoFrame vtable (for reading captured frames) ========
#[repr(C)]
struct IDeckLinkVideoFrameVtbl {
    pub query_interface: unsafe extern "C" fn(*mut c_void, *const u8, *mut *mut c_void) -> HRESULT,
    pub add_ref: unsafe extern "C" fn(*mut c_void) -> u32,
    pub release: unsafe extern "C" fn(*mut c_void) -> u32,
    pub get_width: unsafe extern "C" fn(*mut c_void) -> i64,
    pub get_height: unsafe extern "C" fn(*mut c_void) -> i64,
    pub get_row_bytes: unsafe extern "C" fn(*mut c_void) -> i64,
    pub get_pixel_format: unsafe extern "C" fn(*mut c_void) -> u32,
    pub get_flags: unsafe extern "C" fn(*mut c_void) -> u32,
    pub get_timecode: *const c_void,
    pub get_ancillary_data: *const c_void,
}

// ======== IDeckLinkVideoBuffer vtable ========
#[repr(C)]
struct IDeckLinkVideoBufferVtbl {
    pub query_interface: unsafe extern "C" fn(*mut c_void, *const u8, *mut *mut c_void) -> HRESULT,
    pub add_ref: unsafe extern "C" fn(*mut c_void) -> u32,
    pub release: unsafe extern "C" fn(*mut c_void) -> u32,
    pub get_bytes: unsafe extern "C" fn(*mut c_void, *mut *mut c_void) -> HRESULT,
}

// ======== IDeckLinkInputCallback — Our Rust implementation ========

/// Our callback object — must have a C++ compatible vtable layout.
#[repr(C)]
struct InputCallbackObj {
    vtable: *const InputCallbackVtbl,
    ref_count: std::sync::atomic::AtomicU32,
    sender: std::sync::mpsc::SyncSender<VideoFrame>,
}

#[repr(C)]
struct InputCallbackVtbl {
    pub query_interface: unsafe extern "C" fn(*mut InputCallbackObj, *const u8, *mut *mut c_void) -> HRESULT,
    pub add_ref: unsafe extern "C" fn(*mut InputCallbackObj) -> u32,
    pub release: unsafe extern "C" fn(*mut InputCallbackObj) -> u32,
    pub video_input_format_changed: unsafe extern "C" fn(*mut InputCallbackObj, u32, *mut c_void, u32) -> HRESULT,
    pub video_input_frame_arrived: unsafe extern "C" fn(*mut InputCallbackObj, *mut c_void, *mut c_void) -> HRESULT,
}

// Static vtable for our callback
static INPUT_CALLBACK_VTBL: InputCallbackVtbl = InputCallbackVtbl {
    query_interface: cb_query_interface,
    add_ref: cb_add_ref,
    release: cb_release,
    video_input_format_changed: cb_video_input_format_changed,
    video_input_frame_arrived: cb_video_input_frame_arrived,
};

unsafe extern "C" fn cb_query_interface(
    this: *mut InputCallbackObj,
    _iid: *const u8,
    ppv: *mut *mut c_void,
) -> HRESULT {
    unsafe {
        *ppv = this as *mut c_void;
        cb_add_ref(this);
        S_OK
    }
}

unsafe extern "C" fn cb_add_ref(this: *mut InputCallbackObj) -> u32 {
    unsafe {
        let obj = &*this;
        obj.ref_count.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1
    }
}

unsafe extern "C" fn cb_release(this: *mut InputCallbackObj) -> u32 {
    unsafe {
        let obj = &*this;
        let prev = obj.ref_count.fetch_sub(1, std::sync::atomic::Ordering::SeqCst);
        if prev == 1 {
            let _ = Box::from_raw(this);
            return 0;
        }
        prev - 1
    }
}

unsafe extern "C" fn cb_video_input_format_changed(
    _this: *mut InputCallbackObj,
    _events: u32,
    _new_mode: *mut c_void,
    _flags: u32,
) -> HRESULT {
    println!("[DeckLink Input] Video input format changed");
    S_OK
}

unsafe extern "C" fn cb_video_input_frame_arrived(
    this: *mut InputCallbackObj,
    video_frame: *mut c_void,
    _audio_packet: *mut c_void,
) -> HRESULT {
    unsafe {
        if video_frame.is_null() {
            return S_OK;
        }

        let obj = &*this;

        // Read frame dimensions via IDeckLinkVideoFrame vtable
        let vtable_ptr = *(video_frame as *const *const IDeckLinkVideoFrameVtbl);
        let vtable = &*vtable_ptr;

        let width = (vtable.get_width)(video_frame) as u32;
        let height = (vtable.get_height)(video_frame) as u32;
        let row_bytes = (vtable.get_row_bytes)(video_frame) as u32;

        if width == 0 || height == 0 {
            return S_OK;
        }

        // QI for IDeckLinkVideoBuffer to get pixel data
        let mut buffer_ptr: *mut c_void = ptr::null_mut();
        let res = (vtable.query_interface)(video_frame, IID_VIDEO_BUFFER.as_ptr(), &mut buffer_ptr);
        if res != S_OK || buffer_ptr.is_null() {
            return S_OK;
        }

        let vbuf_vtable = &*(*(buffer_ptr as *const *const IDeckLinkVideoBufferVtbl));
        let mut data_ptr: *mut c_void = ptr::null_mut();
        let res = (vbuf_vtable.get_bytes)(buffer_ptr, &mut data_ptr);
        if res != S_OK || data_ptr.is_null() {
            (vbuf_vtable.release)(buffer_ptr);
            return S_OK;
        }

        // Copy BGRA data
        let data_size = (row_bytes * height) as usize;
        let mut data = vec![0u8; data_size];
        ptr::copy_nonoverlapping(data_ptr as *const u8, data.as_mut_ptr(), data_size);

        // Release the video buffer reference
        (vbuf_vtable.release)(buffer_ptr);

        // Send frame to compositor
        let frame = VideoFrame {
            width,
            height,
            data: Some(Arc::new(data)),
            #[cfg(target_os = "macos")]
            pixel_buffer: None,
            timestamp: Instant::now(),
        };

        let _ = obj.sender.try_send(frame);

        S_OK
    }
}

// ======== High-level DeckLinkInput API ========

pub struct DeckLinkInput {
    input: *mut IDeckLinkInput,
    #[allow(dead_code)]
    callback: *mut InputCallbackObj,
}

unsafe impl Send for DeckLinkInput {}

impl DeckLinkInput {
    /// Start capturing video input on the given DeckLink device.
    /// Returns (DeckLinkInput, Receiver) — the receiver gets VideoFrames from the callback.
    pub fn start(device: &DeckLinkDevice) -> Option<(Self, std::sync::mpsc::Receiver<VideoFrame>)> {
        unsafe {
            // Use IUnknown::QueryInterface on the raw device pointer to get IDeckLinkInput
            let dev_ptr = device.device as *mut c_void;
            let iunknown_vtable = &*(*(dev_ptr as *const *const IUnknownVtbl));

            let mut input_ptr: *mut c_void = ptr::null_mut();
            let res = (iunknown_vtable.query_interface)(dev_ptr, IID_IDECK_LINK_INPUT.as_ptr(), &mut input_ptr);

            if res != S_OK || input_ptr.is_null() {
                eprintln!("[DeckLink Input] Failed to get IDeckLinkInput interface");
                return None;
            }

            let dl_input = input_ptr as *mut IDeckLinkInput;
            let in_vtable = &*(*dl_input).vtable;

            // Enable video input (1080p60, BGRA)
            let res = (in_vtable.enable_video_input)(
                dl_input as *mut c_void,
                BMD_MODE_HD1080P60,
                BMD_FORMAT_8BIT_BGRA,
                BMD_VIDEO_INPUT_FLAG_DEFAULT,
            );
            if res != S_OK {
                eprintln!("[DeckLink Input] EnableVideoInput failed: 0x{:08x}", res);
                (in_vtable.release)(dl_input as *mut c_void);
                return None;
            }

            // Create our callback object
            let (sender, receiver) = std::sync::mpsc::sync_channel::<VideoFrame>(2);
            let callback = Box::into_raw(Box::new(InputCallbackObj {
                vtable: &INPUT_CALLBACK_VTBL,
                ref_count: std::sync::atomic::AtomicU32::new(1),
                sender,
            }));

            // Set callback
            let res = (in_vtable.set_callback)(dl_input as *mut c_void, callback as *mut c_void);
            if res != S_OK {
                eprintln!("[DeckLink Input] SetCallback failed: 0x{:08x}", res);
                (in_vtable.disable_video_input)(dl_input as *mut c_void);
                (in_vtable.release)(dl_input as *mut c_void);
                let _ = Box::from_raw(callback);
                return None;
            }

            // Start streams
            let res = (in_vtable.start_streams)(dl_input as *mut c_void);
            if res != S_OK {
                eprintln!("[DeckLink Input] StartStreams failed: 0x{:08x}", res);
                (in_vtable.set_callback)(dl_input as *mut c_void, ptr::null_mut());
                (in_vtable.disable_video_input)(dl_input as *mut c_void);
                (in_vtable.release)(dl_input as *mut c_void);
                let _ = Box::from_raw(callback);
                return None;
            }

            println!("[DeckLink Input] ✅ Monitoring started (1080p60 BGRA)");
            Some((
                DeckLinkInput {
                    input: dl_input,
                    callback,
                },
                receiver,
            ))
        }
    }

    pub fn stop(&mut self) {
        unsafe {
            if !self.input.is_null() {
                let vtable = &*(*self.input).vtable;
                (vtable.stop_streams)(self.input as *mut c_void);
                (vtable.set_callback)(self.input as *mut c_void, ptr::null_mut());
                (vtable.disable_video_input)(self.input as *mut c_void);
                println!("[DeckLink Input] Monitoring stopped");
            }
        }
    }
}

impl Drop for DeckLinkInput {
    fn drop(&mut self) {
        self.stop();
        unsafe {
            if !self.input.is_null() {
                let vtable = &*(*self.input).vtable;
                (vtable.release)(self.input as *mut c_void);
                self.input = ptr::null_mut();
            }
        }
    }
}
