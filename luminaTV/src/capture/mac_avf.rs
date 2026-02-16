use std::sync::{Arc, Mutex};
use std::ffi::c_void;
use std::ptr::NonNull;
use tokio::sync::mpsc;



use objc2::rc::Retained;
use objc2_foundation::{MainThreadMarker, NSObject, NSString, NSDictionary, NSNumber, NSCopying};

use objc2_av_foundation::{
    AVCaptureDevice, AVCaptureDeviceInput, AVCaptureSession, AVCaptureVideoDataOutput,
    AVCaptureSessionPresetHigh, AVCaptureOutput, AVCaptureConnection,
    AVCaptureVideoDataOutputSampleBufferDelegate,
};
use objc2_core_media::CMSampleBuffer;
use objc2_core_video::{CVPixelBuffer, kCVPixelFormatType_32BGRA};
use objc2::{define_class, msg_send, ClassType, DeclaredClass};
use objc2::runtime::{ProtocolObject, NSObjectProtocol};



use dispatch2::DispatchRetained;

// Need FFI for dispatch queue and CVPixelBuffer
#[link(name = "CoreVideo", kind = "framework")]

unsafe extern "C" {

    fn dispatch_queue_create(label: *const std::os::raw::c_char, attr: *const std::os::raw::c_void) -> *mut std::ffi::c_void;
    
    // CVPixelBuffer FFI
    fn CVPixelBufferGetWidth(pixelBuffer: *mut c_void) -> usize;

    fn CVPixelBufferGetHeight(pixelBuffer: *mut c_void) -> usize;
    fn CVPixelBufferLockBaseAddress(pixelBuffer: *mut c_void, lockFlags: u64) -> i32;
    fn CVPixelBufferUnlockBaseAddress(pixelBuffer: *mut c_void, lockFlags: u64) -> i32;
    fn CVPixelBufferGetBaseAddress(pixelBuffer: *mut c_void) -> *mut c_void;
    fn CVPixelBufferGetBytesPerRow(pixelBuffer: *mut c_void) -> usize;
}

const K_CVPIXEL_BUFFER_LOCK_READ_ONLY: u64 = 0x00000001;

fn extract_bgra_from_pixel_buffer(pixel_buffer: &CVPixelBuffer) -> Option<(u32, u32, Vec<u8>)> {
    // Cast to void* for FFI
    let pb_ptr = pixel_buffer as *const CVPixelBuffer as *mut c_void;

    unsafe {
        if CVPixelBufferLockBaseAddress(pb_ptr, K_CVPIXEL_BUFFER_LOCK_READ_ONLY) != 0 {
            eprintln!("[MacAVF] Failed to lock pixel buffer");
            return None;
        }
        
        // Ensure unlock happens
        struct UnlockGuard(*mut c_void);
        impl Drop for UnlockGuard {
            fn drop(&mut self) {
                unsafe { CVPixelBufferUnlockBaseAddress(self.0, K_CVPIXEL_BUFFER_LOCK_READ_ONLY); }
            }
        }
        let _guard = UnlockGuard(pb_ptr);

        let width = CVPixelBufferGetWidth(pb_ptr) as u32;
        let height = CVPixelBufferGetHeight(pb_ptr) as u32;
        let base_address = CVPixelBufferGetBaseAddress(pb_ptr) as *const u8;
        let bytes_per_row = CVPixelBufferGetBytesPerRow(pb_ptr);

        if width == 0 || height == 0 || base_address.is_null() {
            return None;
        }

        let dst_stride = (width * 4) as usize;
        let data_len = dst_stride * height as usize;
        let mut data = vec![0u8; data_len];

        // Copy row by row to handle padding
        for y in 0..height {
            let src_offset = (y as usize) * bytes_per_row;
            let dst_offset = (y as usize) * dst_stride;
            let row_src = std::slice::from_raw_parts(base_address.add(src_offset), dst_stride);
            data[dst_offset..dst_offset + dst_stride].copy_from_slice(row_src);
        }

        Some((width, height, data))
    }
}









use crate::core::VideoFrame;
// Removed import of private function




// wait, implementing delegate correctly with state in rust `objc2` is slightly verbose.
// Let's use the simpler pattern: just implement the logic and don't worry about Ivars yet if we can't easily.
// Actually, to pass `tx` to the delegate, we need `Ivar`.

// Ivar imports removed


#[derive(Default)]
pub struct CameraDelegateIvars {
    tx: Mutex<Option<mpsc::Sender<VideoFrame>>>,
}

define_class!(
    #[unsafe(super(NSObject))]
    #[name = "LuminaCameraDelegateImpl"]
    #[ivars = CameraDelegateIvars]
    struct CameraDelegateImpl;

    impl CameraDelegateImpl {
        #[unsafe(method(initWithTx:))]
        unsafe fn init_with_tx(&self, tx: *mut c_void) -> Option<&Self> {
            let this: Option<&Self> = msg_send![super(self), init];
            if let Some(this) = this {
                 let tx_box = unsafe { Box::from_raw(tx as *mut mpsc::Sender<VideoFrame>) };


                 // Use Mutex
                 *this.ivars().tx.lock().unwrap() = Some(*tx_box);
            }
            this
        }

        #[unsafe(method(captureOutput:didOutputSampleBuffer:fromConnection:))]
        unsafe fn capture_output_did_output_sample_buffer_from_connection(

            &self,
            _output: &AVCaptureOutput,
            sample_buffer: &CMSampleBuffer,
            _connection: &AVCaptureConnection,
        ) {
            // Updated for new SDK or deprecated warning suppressed?
            // CMSampleBufferGetImageBuffer is deprecated, use CMSampleBuffer::image_buffer if available.
            // But we imported CMSampleBufferGetImageBuffer.
            // If we use `CMSampleBuffer::image_buffer` we need to check if it returns Option<&CVPixelBuffer>.
            // Assuming it does or we use the C function.
            // Let's use the C function for now as it worked (despite warning).
            #[allow(deprecated)]
            let pixel_buffer = match unsafe { CMSampleBuffer::image_buffer(sample_buffer) } {
                Some(pb) => pb,
                None => return,
            };

            
            if let Some((width, height, data)) = extract_bgra_from_pixel_buffer(&pixel_buffer) {
                use std::sync::Once;
                static START: Once = Once::new();
                START.call_once(|| {
                    println!("[MacAVF] First frame captured: {}x{} (Bytes: {})", width, height, data.len());
                });

                let frame = VideoFrame {

                    width, height,
                    data: Arc::new(data),
                    timestamp: std::time::Instant::now(),
                };
                
                // Send
                if let Some(tx) = self.ivars().tx.lock().unwrap().as_ref() {
                    let _ = tx.try_send(frame);
                }
            } else {
                 // Debug: buffer extraction failed
                 // eprintln!("[MacAVF] Failed to extract BGRA");
            }
        }
    }
);

unsafe impl AVCaptureVideoDataOutputSampleBufferDelegate for CameraDelegateImpl {}
unsafe impl NSObjectProtocol for CameraDelegateImpl {}


impl CameraDelegateImpl {
    pub fn new(tx: mpsc::Sender<VideoFrame>) -> Retained<Self> {
        let tx_ptr = Box::into_raw(Box::new(tx));
        unsafe { msg_send![msg_send![Self::class(), alloc], initWithTx: tx_ptr as *mut c_void] }
    }

}









pub struct CameraSession {
    session: Retained<AVCaptureSession>,
    #[allow(dead_code)]
    delegate: Retained<CameraDelegateImpl>,

    _queue: dispatch2::DispatchRetained<dispatch2::DispatchQueue>, // Keep queue alive
}






impl CameraSession {
    pub fn start(device_unique_id: &str, tx: mpsc::Sender<VideoFrame>) -> Result<Self, String> {
        let _mtm = MainThreadMarker::new().ok_or("Must run on main thread")?;
        
        let device = unsafe { AVCaptureDevice::deviceWithUniqueID(&NSString::from_str(device_unique_id)) }
            .ok_or("Device not found")?;

        let input = unsafe { AVCaptureDeviceInput::deviceInputWithDevice_error(&device) }
            .map_err(|e| format!("Input error: {}", e))?;

        let session = unsafe { AVCaptureSession::new() };
        unsafe { session.beginConfiguration() };
        
        if unsafe { session.canAddInput(&input) } {
             unsafe { session.addInput(&input) };
        } else {
             return Err("Cannot add input".into());
        }

        let output = unsafe { AVCaptureVideoDataOutput::new() };
        
        // Set Pixel Format to BGRA
        // kCVPixelBufferPixelFormatTypeKey is CFString, cast to NSString
        let key_cf = unsafe { objc2_core_video::kCVPixelBufferPixelFormatTypeKey };
        let key_ref: &NSString = unsafe { &*(key_cf as *const _ as *const NSString) };
        let key: Retained<NSString> = key_ref.copy();
        
        // Value: NSNumber is AnyObject
        let val: Retained<NSNumber> = NSNumber::numberWithUnsignedInt(kCVPixelFormatType_32BGRA);
        
        let val_obj: Retained<objc2::runtime::AnyObject> = unsafe { std::mem::transmute(val) };

        let keys = [&*key];
        let objects = [&*val_obj];
        let settings = NSDictionary::from_slices(&keys, &objects);

        
        unsafe { output.setVideoSettings(Some(&settings)) };

        
        // Set Delegate
        let delegate = CameraDelegateImpl::new(tx);
        // Create Serial Queue for Capture
        let label = std::ffi::CString::new("com.lumina.camera.capture").unwrap();
        let queue_ptr = unsafe { dispatch_queue_create(label.as_ptr(), std::ptr::null()) };
        
        // dispatch2 queue creation
        let queue_nonnull = NonNull::new(queue_ptr as *mut _).ok_or("Failed to create queue")?;
        let queue = unsafe { dispatch2::DispatchRetained::from_raw(queue_nonnull) };

        let protocol_obj = ProtocolObject::from_ref(&*delegate);
        
        unsafe { output.setSampleBufferDelegate_queue(Some(protocol_obj), Some(&queue)) };


        if unsafe { session.canAddOutput(&output) } {
            unsafe { session.addOutput(&output) };
        }

        unsafe { session.setSessionPreset(AVCaptureSessionPresetHigh) }; 
        unsafe { session.commitConfiguration() };
        
        unsafe { session.startRunning() };
        println!("[MacAVF] Camera session started for device: {}", device_unique_id);


        Ok(CameraSession { session, delegate, _queue: queue })

    }


    pub fn stop(&self) {
        unsafe { self.session.stopRunning() };
    }
}

