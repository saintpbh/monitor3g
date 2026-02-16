use std::ffi::c_void;
use std::ptr;
use crate::output::decklink_sys::{HRESULT, CFStringRef};
use crate::core::VideoFrame;

// Constants from DeckLink SDK
const S_OK: HRESULT = 0;
const BMD_FORMAT_8BIT_BGRA: u32 = 0x42475241; // 'BGRA'
const BMD_MODE_HD1080P60: u32 = 0x48703630;   // 'Hp60'
const BMD_VIDEO_OUTPUT_FLAG_DEFAULT: u32 = 0;

// Interface IDs
const IID_IDECK_LINK_OUTPUT: [u8; 16] = [0x1A,0x80,0x77,0xF1,0x9F,0xE2,0x45,0x33,0x81,0x47,0x22,0x94,0x30,0x5E,0x25,0x3F];
const IID_IDECK_LINK_VIDEO_BUFFER: [u8; 16] = [0xCC,0xB4,0xB6,0x4A,0x5C,0x86,0x4E,0x02,0xB7,0x78,0x88,0x5D,0x35,0x27,0x09,0xFE];

#[repr(C)]
struct IDeckLinkIteratorVtbl {
    pub query_interface: unsafe extern "C" fn(*mut c_void, *const u8, *mut *mut c_void) -> HRESULT,
    pub add_ref: unsafe extern "C" fn(*mut c_void) -> u32,
    pub release: unsafe extern "C" fn(*mut c_void) -> u32,
    pub next: unsafe extern "C" fn(*mut c_void, *mut *mut IDeckLink) -> HRESULT,
}

#[repr(C)]
struct IDeckLinkVtbl {
    pub query_interface: unsafe extern "C" fn(*mut c_void, *const u8, *mut *mut c_void) -> HRESULT,
    pub add_ref: unsafe extern "C" fn(*mut c_void) -> u32,
    pub release: unsafe extern "C" fn(*mut c_void) -> u32,
    pub get_model_name: unsafe extern "C" fn(*mut c_void, *mut CFStringRef) -> HRESULT,
    pub get_display_name: unsafe extern "C" fn(*mut c_void, *mut CFStringRef) -> HRESULT,
}

#[repr(C)]
struct IDeckLinkOutputVtbl {
    pub query_interface: unsafe extern "C" fn(*mut c_void, *const u8, *mut *mut c_void) -> HRESULT,
    pub add_ref: unsafe extern "C" fn(*mut c_void) -> u32,
    pub release: unsafe extern "C" fn(*mut c_void) -> u32,
    pub does_support_video_mode: *const c_void,
    pub get_display_mode: *const c_void,
    pub get_display_mode_iterator: *const c_void,
    pub set_screen_preview_callback: *const c_void,
    pub enable_video_output: unsafe extern "C" fn(*mut c_void, u32, u32) -> HRESULT,
    pub disable_video_output: unsafe extern "C" fn(*mut c_void) -> HRESULT,
    pub create_video_frame: unsafe extern "C" fn(*mut c_void, i32, i32, i32, u32, u32, *mut *mut IDeckLinkMutableVideoFrame) -> HRESULT,
    pub create_video_frame_with_buffer: *const c_void,
    pub row_bytes_for_pixel_format: *const c_void,
    pub create_ancillary_data: *const c_void,
    pub display_video_frame_sync: unsafe extern "C" fn(*mut c_void, *mut IDeckLinkVideoFrame) -> HRESULT,
}

#[repr(C)]
struct IDeckLinkVideoFrameVtbl {
    pub query_interface: unsafe extern "C" fn(*mut c_void, *const u8, *mut *mut c_void) -> HRESULT,
    pub add_ref: unsafe extern "C" fn(*mut c_void) -> u32,
    pub release: unsafe extern "C" fn(*mut c_void) -> u32,
}

#[repr(C)]
struct IDeckLinkVideoBufferVtbl {
    pub query_interface: unsafe extern "C" fn(*mut c_void, *const u8, *mut *mut c_void) -> HRESULT,
    pub add_ref: unsafe extern "C" fn(*mut c_void) -> u32,
    pub release: unsafe extern "C" fn(*mut c_void) -> u32,
    pub get_bytes: unsafe extern "C" fn(*mut c_void, *mut *mut c_void) -> HRESULT,
}

#[repr(C)] pub struct IDeckLinkIterator { vtable: *const IDeckLinkIteratorVtbl }
#[repr(C)] pub struct IDeckLink { vtable: *const IDeckLinkVtbl }
#[repr(C)] pub struct IDeckLinkOutput { vtable: *const IDeckLinkOutputVtbl }
#[repr(C)] pub struct IDeckLinkVideoFrame { vtable: *const IDeckLinkVideoFrameVtbl }
#[repr(C)] pub struct IDeckLinkMutableVideoFrame { vtable: *const IDeckLinkVideoFrameVtbl }
#[repr(C)] pub struct IDeckLinkVideoBuffer { vtable: *const IDeckLinkVideoBufferVtbl }

// Safety: DeckLink interfaces are thread-safe on Mac
unsafe impl Send for IDeckLinkIterator {}
unsafe impl Send for IDeckLink {}
unsafe impl Send for IDeckLinkOutput {}
unsafe impl Send for IDeckLinkVideoFrame {}
unsafe impl Send for IDeckLinkMutableVideoFrame {}
unsafe impl Send for IDeckLinkVideoBuffer {}

unsafe extern "C" {
    fn CreateDeckLinkIteratorInstance() -> *mut IDeckLinkIterator;
}

pub struct DeckLinkDevice {
    pub name: String,
    pub index: usize,
    pub device: *mut IDeckLink,
}

unsafe impl Send for DeckLinkDevice {}

impl Drop for DeckLinkDevice {
    fn drop(&mut self) {
        unsafe {
            if !self.device.is_null() {
                ((&*( *self.device ).vtable).release)(self.device as *mut c_void);
            }
        }
    }
}

pub struct DeckLinkOutput {
    output: *mut IDeckLinkOutput,
    width: u32,
    height: u32,
}

unsafe impl Send for DeckLinkOutput {}

impl DeckLinkOutput {
    pub fn new(device: &DeckLinkDevice) -> Option<Self> {
        unsafe {
            let mut output: *mut c_void = ptr::null_mut();
            let vtable = &*(*device.device).vtable;
            let res = (vtable.query_interface)(device.device as *mut c_void, IID_IDECK_LINK_OUTPUT.as_ptr(), &mut output);
            
            if res != S_OK || output.is_null() {
                return None;
            }

            let dl_output = output as *mut IDeckLinkOutput;
            let out_vtable = &*(*dl_output).vtable;
            
            let res = (out_vtable.enable_video_output)(dl_output as *mut c_void, BMD_MODE_HD1080P60, BMD_VIDEO_OUTPUT_FLAG_DEFAULT);
            if res != S_OK {
                ((&*( *dl_output ).vtable).release)(dl_output as *mut c_void);
                return None;
            }

            println!("[DeckLink] Output enabled (1080p60)");
            Some(DeckLinkOutput { output: dl_output, width: 1920, height: 1080 })
        }
    }

    pub fn send_frame(&mut self, frame: &VideoFrame) {
        if frame.width != self.width || frame.height != self.height {
            // Resize or skip? For simple implementation, skip.
            return;
        }

        unsafe {
            let out_vtable = &*(*self.output).vtable;
            let mut dl_frame: *mut IDeckLinkMutableVideoFrame = ptr::null_mut();
            
            let res = (out_vtable.create_video_frame)(
                self.output as *mut c_void,
                self.width as i32,
                self.height as i32,
                (self.width * 4) as i32,
                BMD_FORMAT_8BIT_BGRA,
                0, // flags
                &mut dl_frame
            );

            if res == S_OK && !dl_frame.is_null() {
                // Get buffer
                let mut buffer_ptr: *mut c_void = ptr::null_mut();
                let mut dl_buffer: *mut c_void = ptr::null_mut();
                
                let frame_vtable = &*(*dl_frame).vtable;
                let res_buf = (frame_vtable.query_interface)(dl_frame as *mut c_void, IID_IDECK_LINK_VIDEO_BUFFER.as_ptr(), &mut dl_buffer);
                
                if res_buf == S_OK && !dl_buffer.is_null() {
                    let dl_buffer_iface = dl_buffer as *mut IDeckLinkVideoBuffer;
                    let buf_vtable = &*(*dl_buffer_iface).vtable;
                    
                    if (buf_vtable.get_bytes)(dl_buffer as *mut c_void, &mut buffer_ptr) == S_OK {
                        ptr::copy_nonoverlapping(frame.data.as_ptr(), buffer_ptr as *mut u8, (self.width * self.height * 4) as usize);
                        
                        // Display Sync
                        (out_vtable.display_video_frame_sync)(self.output as *mut c_void, dl_frame as *mut IDeckLinkVideoFrame);
                    }
                    (buf_vtable.release)(dl_buffer as *mut c_void);
                }
                (frame_vtable.release)(dl_frame as *mut c_void);
            }
        }
    }
}

impl Drop for DeckLinkOutput {
    fn drop(&mut self) {
        unsafe {
            if !self.output.is_null() {
                let vtable = &*(*self.output).vtable;
                (vtable.disable_video_output)(self.output as *mut c_void);
                (vtable.release)(self.output as *mut c_void);
            }
        }
    }
}

pub struct DeckLinkManager;

impl DeckLinkManager {
    pub fn enumerate_devices() -> Vec<DeckLinkDevice> {
        let mut devices = Vec::new();
        unsafe {
            let iterator = CreateDeckLinkIteratorInstance();
            if iterator.is_null() { return devices; }

            let mut device: *mut IDeckLink = ptr::null_mut();
            let mut index = 0;
            loop {
                let vtable = &*(*iterator).vtable;
                if (vtable.next)(iterator as *mut c_void, &mut device) != S_OK { break; }
                
                let mut name_cf: CFStringRef = ptr::null_mut();
                let dev_vtable = &*(*device).vtable;
                (dev_vtable.get_display_name)(device as *mut c_void, &mut name_cf);
                
                let name = cf_string_to_string(name_cf);
                println!("[DeckLink] Found device: {}", name);
                
                devices.push(DeckLinkDevice { name, index, device });
                index += 1;
            }
            ((&*(*iterator).vtable).release)(iterator as *mut c_void);
        }
        devices
    }

    pub fn get_device_by_index(index: usize) -> Option<DeckLinkDevice> {
        unsafe {
            let iterator = CreateDeckLinkIteratorInstance();
            if iterator.is_null() { return None; }

            let mut device: *mut IDeckLink = ptr::null_mut();
            let mut current = 0;
            let mut found = None;

            loop {
                let vtable = &*(*iterator).vtable;
                if (vtable.next)(iterator as *mut c_void, &mut device) != S_OK { break; }
                
                if current == index {
                    let mut name_cf: CFStringRef = ptr::null_mut();
                    let dev_vtable = &*(*device).vtable;
                    (dev_vtable.get_display_name)(device as *mut c_void, &mut name_cf);
                    found = Some(DeckLinkDevice { name: cf_string_to_string(name_cf), index, device });
                    break;
                } else {
                    ((&*(*device).vtable).release)(device as *mut c_void);
                }
                current += 1;
            }
            ((&*(*iterator).vtable).release)(iterator as *mut c_void);
            found
        }
    }
}

unsafe fn cf_string_to_string(cf: CFStringRef) -> String {
    if cf.is_null() { return "Unknown".to_string(); }
    use objc2::rc::Retained;
    use objc2_foundation::NSString;
    let ns_string = cf as *const NSString;
    let retained = unsafe { Retained::retain(ns_string as *mut NSString) };
    match retained {
        Some(s) => s.to_string(),
        None => "Invalid String".to_string(),
    }
}




