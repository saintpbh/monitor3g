use std::sync::{Arc};
use std::ffi::c_void;
use std::ptr::NonNull;
use tokio::sync::mpsc;

use objc2::rc::Retained;
use objc2_foundation::{NSString, NSDictionary, NSNumber, NSCopying};

use objc2_av_foundation::{
    AVCaptureDevice, AVCaptureDeviceInput, AVCaptureSession, AVCaptureVideoDataOutput,
    AVCaptureSessionPresetHigh, AVCaptureVideoDataOutputSampleBufferDelegate,
};
use objc2_core_video::{kCVPixelFormatType_32BGRA};
// use objc2::{msg_send};
use objc2::runtime::{ProtocolObject, Bool, AnyObject};

// use dispatch2::DispatchRetained;
use block2::RcBlock;
use objc2_av_foundation::AVMediaTypeVideo;
use std::os::raw::{c_int};

use crate::core::VideoFrame;

// External C functions from camera_delegate.m
unsafe extern "C" {
    fn create_lumina_delegate(
        callback: extern "C" fn(*const u8, usize, c_int, c_int, usize, *mut c_void),
        ctx: *mut c_void
    ) -> *mut c_void;
}

// Callback implementation called from C
extern "C" fn camera_frame_callback(
    data: *const u8,
    len: usize,
    width: c_int,
    height: c_int,
    bytes_per_row: usize,
    ctx: *mut c_void
) {
    if ctx.is_null() { return; }
    // Cast ctx back to Sender. We passed Box::into_raw(Box::new(tx)).
    // Wait, we pass the pointer to the HEAP allocated Sender.
    // We must NOT drop it here. Just use it.
    let tx = unsafe { &*(ctx as *const mpsc::Sender<VideoFrame>) };

    // Basic logging throttling
    use std::sync::atomic::{AtomicUsize, Ordering};
    static FRAME_COUNT: AtomicUsize = AtomicUsize::new(0);
    let count = FRAME_COUNT.fetch_add(1, Ordering::Relaxed);
    
    if count == 0 {
        println!("[MacAVF] First frame captured via C delegate! {}x{}", width, height);
    }
    if count % 300 == 0 {
        println!("[MacAVF] Frame #{} ({} bytes, row={})", count, len, bytes_per_row);
    }

    // Process frame
    // We expect BGRA (requested in settings).
    // The data pointer is valid ONLY during this callback (locked base address).
    // We must copy it.
    let data_slice = unsafe { std::slice::from_raw_parts(data, len) };
    let vec = data_slice.to_vec();

    let frame = VideoFrame {
        width: width as u32,
        height: height as u32,
        data: Some(Arc::new(vec)),
        #[cfg(target_os = "macos")]
        pixel_buffer: None,
        timestamp: std::time::Instant::now(),
    };
    
    // Send
    let _ = tx.try_send(frame);
}

pub fn request_permission() {
    unsafe {
        let media_type = AVMediaTypeVideo.expect("AVMediaTypeVideo not found");
        let status = AVCaptureDevice::authorizationStatusForMediaType(media_type);
        println!("[MacAVF] Startup Auth Status: {}", status.0);

        let handler = RcBlock::new(|granted: Bool| {
            if granted.as_bool() {
                println!("[MacAVF] Camera Access GRANTED by user/system.");
            } else {
                eprintln!("[MacAVF] Camera Access DENIED by user/system.");
            }
        });
        
        AVCaptureDevice::requestAccessForMediaType_completionHandler(media_type, &handler);
    }
}

pub fn list_devices() -> Vec<(String, String)> {
    let mut devices = Vec::new();
    unsafe {
        let media_type = AVMediaTypeVideo.expect("AVMediaTypeVideo not found");
        #[allow(deprecated)]
        let all_devices = AVCaptureDevice::devicesWithMediaType(media_type);
        for device in all_devices {
             let unique_id = device.uniqueID().to_string();
             let name = device.localizedName().to_string();
             devices.push((unique_id, name));
        }
    }
    devices
}

#[link(name = "CoreVideo", kind = "framework")]
unsafe extern "C" {
    fn dispatch_queue_create(label: *const std::os::raw::c_char, attr: *const std::os::raw::c_void) -> *mut std::ffi::c_void;
}

pub struct CameraSession {
    session: Retained<AVCaptureSession>,
    _delegate: Retained<AnyObject>, // Hold delegate opaquely
    _tx_ptr: Box<mpsc::Sender<VideoFrame>>, // Keep sender alive
    _queue: dispatch2::DispatchRetained<dispatch2::DispatchQueue>, // Keep queue alive
}

impl CameraSession {
    pub fn start(device_unique_id: &str, tx: mpsc::Sender<VideoFrame>) -> Result<Self, String> {
        // Check auth status first
        unsafe {
            let media_type = AVMediaTypeVideo.expect("AVMediaTypeVideo not found");
            let status = AVCaptureDevice::authorizationStatusForMediaType(media_type);
            println!("[MacAVF] Auth status for video: {}", status.0);
            
            if status.0 != 3 {
                eprintln!("[MacAVF] WARNING: Camera not authorized (Status: {}). Requesting permission...", status.0);
                request_permission();
            }
        }

        println!("[MacAVF] Finding device: {}", device_unique_id);
        let device = unsafe { AVCaptureDevice::deviceWithUniqueID(&NSString::from_str(device_unique_id)) }
            .ok_or_else(|| {
                eprintln!("[MacAVF] Device detection failed");
                "Device not found".to_string()
            })?;

        println!("[MacAVF] Creating input for: {}", unsafe { device.localizedName().to_string() });
        let input = unsafe { AVCaptureDeviceInput::deviceInputWithDevice_error(&device) }
            .map_err(|e| format!("Input error: {}", e))?;

        println!("[MacAVF] Creating session...");
        let session = unsafe { AVCaptureSession::new() };
        unsafe { session.beginConfiguration() };
        
        if unsafe { session.canAddInput(&input) } {
             unsafe { session.addInput(&input) };
             println!("[MacAVF] Input added.");
        } else {
             return Err("Cannot add input".into());
        }

        let output = unsafe { AVCaptureVideoDataOutput::new() };
        
        // Settings for BGRA
        let key_cf = unsafe { objc2_core_video::kCVPixelBufferPixelFormatTypeKey };
        let key_ref: &NSString = unsafe { &*(key_cf as *const _ as *const NSString) };
        let key: Retained<NSString> = key_ref.copy();
        let val: Retained<NSNumber> = NSNumber::numberWithUnsignedInt(kCVPixelFormatType_32BGRA);
        let val_obj: Retained<AnyObject> = unsafe { std::mem::transmute(val) };
        let keys = [&*key];
        let objects = [&*val_obj];
        let settings = NSDictionary::from_slices(&keys, &objects);

        unsafe { output.setVideoSettings(Some(&settings)) };

        
        // Create delegate via C
        // We box the Sender to keep it on heap stable address.
        // We pass the raw pointer to C.
        // We MUST store the Box in CameraSession to keep it alive.
        let tx_box = Box::new(tx);
        let tx_ptr = Box::into_raw(tx_box);
        
        let delegate_ptr = unsafe { create_lumina_delegate(camera_frame_callback, tx_ptr as *mut c_void) };
        let delegate: Retained<AnyObject> = unsafe { Retained::from_raw(delegate_ptr as *mut AnyObject) }.unwrap();
        
        // Create Serial Queue for Capture
        let label = std::ffi::CString::new("com.lumina.camera.capture").unwrap();
        let queue_ptr = unsafe { dispatch_queue_create(label.as_ptr(), std::ptr::null()) };
        let queue_nonnull = NonNull::new(queue_ptr as *mut _).ok_or("Failed to create queue")?;
        let queue = unsafe { dispatch2::DispatchRetained::from_raw(queue_nonnull) };

        // ProtocolObject::from_ref requires the trait to be implemented for the type.
        // But delegate is AnyObject.
        // We can cast AnyObject to ProtocolObject<dyn AVCaptureVideoDataOutputSampleBufferDelegate>?
        // No, 'AVCaptureVideoDataOutputSampleBufferDelegate' is a trait in objc2-av-foundation properly?
        // Actually, setSampleBufferDelegate expects `Option<&ProtocolObject<dyn AVCaptureVideoDataOutputSampleBufferDelegate>>`.
        // In objc2 0.6, `ProtocolObject` wraps `Id`.
        // We need to verify if `AnyObject` satisfies it.
        // We can likely just transmute or cast, as we know the ObjC object implements the protocol.
        
        // Safe way: ProtocolObject can be created from Id if we claim it implements it.
        // But the trait is not implemented for AnyObject in Rust.
        // We can use `ProtocolObject::from_id_unchecked` if available? 
        // Or `transmute`.
        let protocol_obj: &ProtocolObject<dyn AVCaptureVideoDataOutputSampleBufferDelegate> = unsafe { std::mem::transmute(&*delegate) };

        unsafe { output.setSampleBufferDelegate_queue(Some(protocol_obj), Some(&queue)) };

        if unsafe { session.canAddOutput(&output) } {
            unsafe { session.addOutput(&output) };
        } else {
             // Clean up
             unsafe { let _ = Box::from_raw(tx_ptr); }
             return Err("Cannot add output".into());
        }

        unsafe { session.setSessionPreset(AVCaptureSessionPresetHigh) }; 
        unsafe { session.commitConfiguration() };
        
        unsafe { session.startRunning() };
        println!("[MacAVF] Camera session started (Native C Delegate) for device: {}", device_unique_id);
        
        // Reconstruct box for ownership storage (keep alive)
        let tx_box = unsafe { Box::from_raw(tx_ptr) };

        Ok(CameraSession {
            session,
            _delegate: delegate,
            _tx_ptr: tx_box,
            _queue: queue,
        })
    }

    pub fn stop(&self) {
        unsafe { self.session.stopRunning() };
    }
}
