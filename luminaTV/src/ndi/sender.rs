use crate::ndi::ffi;
use crate::core::VideoFrame;
use std::ffi::CString;
use std::ptr;

/// NDI Sender wrapper that manages the lifecycle of an NDI send instance.
pub struct NdiSender {
    instance: ffi::NDIlib_send_instance_t,
}

// Raw pointers in NdiSender are only accessed from a single thread (the compositor thread).
unsafe impl Send for NdiSender {}

impl NdiSender {
    /// Create a new NDI sender with the given name.
    pub fn new(name: &str) -> Option<Self> {
        let c_name = CString::new(name).ok()?;

        let send_settings = ffi::NDIlib_send_create_t {
            p_ndi_name: c_name.as_ptr(),
            p_groups: ptr::null(),
            clock_video: true,
            clock_audio: true,
        };

        let instance = unsafe { ffi::NDIlib_send_create(&send_settings) };
        if instance.is_null() {
            eprintln!("[NDI Sender] Failed to create NDI send instance.");
            return None;
        }

        println!("[NDI Sender] Created NDI output: '{}'", name);
        Some(Self { instance })
    }

    /// Send a VideoFrame over NDI using BGRA format.
    pub fn send_frame(&self, frame: &VideoFrame) {
        let line_stride = (frame.width * 4) as i32; // BGRA = 4 bytes per pixel

        let ndi_frame = ffi::NDIlib_video_frame_v2_t {
            xres: frame.width as i32,
            yres: frame.height as i32,
            FourCC: ffi::NDIlib_FourCC_video_type_e_NDIlib_FourCC_video_type_BGRA,
            frame_rate_N: 30000,
            frame_rate_D: 1001,
            picture_aspect_ratio: 0.0, // auto
            frame_format_type: ffi::NDIlib_frame_format_type_e_NDIlib_frame_format_type_progressive,
            timecode: ffi::NDIlib_send_timecode_synthesize,
            p_data: frame.data.as_ptr() as *mut u8,
            __bindgen_anon_1: ffi::NDIlib_video_frame_v2_t__bindgen_ty_1 {
                line_stride_in_bytes: line_stride,
            },
            p_metadata: ptr::null(),
            timestamp: 0,
        };

        unsafe {
            ffi::NDIlib_send_send_video_v2(self.instance, &ndi_frame);
        }
    }
}

impl Drop for NdiSender {
    fn drop(&mut self) {
        println!("[NDI Sender] Destroying NDI send instance.");
        unsafe {
            ffi::NDIlib_send_destroy(self.instance);
        }
    }
}
