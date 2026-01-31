# DeckLink SDK Implementation Best Practices (macOS)

## Issue: Persistent Green Screen / Zeroed Buffer Access

### Root Cause
On macOS, the DeckLink SDK requires explicit memory mapping and synchronization for video frame buffers. Simply calling `GetBytes()` on an `IDeckLinkMutableVideoFrame` or `IDeckLinkVideoFrame` is insufficient and often returns a nullptr or an uninitialized buffer (leading to a green screen in YUV color space, where 0,0,0 translates to green).

### Mandatory Pattern
To properly access video buffer memory on macOS, the `IDeckLinkVideoBuffer` interface must be used with `StartAccess` and `EndAccess` guards.

#### 1. Writing to a Frame (Sources)
When generating or copying data into a frame for SDI output:

```cpp
// 1. Query the IDeckLinkVideoBuffer interface
IDeckLinkVideoBuffer* videoBuffer = nullptr;
if (frame->QueryInterface(IID_IDeckLinkVideoBuffer, (void**)&videoBuffer) == S_OK) {
    void* buffer = nullptr;
    
    // 2. Start write access
    if (videoBuffer->StartAccess(bmdBufferAccessWrite) == S_OK) {
        // 3. Get the raw pointer
        if (videoBuffer->GetBytes(&buffer) == S_OK && buffer) {
            // 4. Perform memory operations (memcpy, pattern gen, etc.)
            long expectedSize = frame->GetRowBytes() * frame->GetHeight();
            memcpy(buffer, sourceData, expectedSize);
        }
        
        // 5. End write access IMMEDIATELY after modification
        videoBuffer->EndAccess(bmdBufferAccessWrite);
    }
    
    // 6. Release the interface
    videoBuffer->Release();
}
```

#### 2. Reading from a Frame (UI Previews)
When reading frame data for UI display or analysis:

```cpp
IDeckLinkVideoBuffer* videoBuffer = nullptr;
if (frame->QueryInterface(IID_IDeckLinkVideoBuffer, (void**)&videoBuffer) == S_OK) {
    void* buffer = nullptr;
    
    // Use bmdBufferAccessRead for reading
    if (videoBuffer->StartAccess(bmdBufferAccessRead) == S_OK) {
        if (videoBuffer->GetBytes(&buffer) == S_OK && buffer) {
            // Process buffer data...
        }
        videoBuffer->EndAccess(bmdBufferAccessRead);
    }
    videoBuffer->Release();
}
```

### Key Learnings
1. **Platform Specificity**: This error is highly specific to macOS. While other systems (Windows/Linux) might allow direct access via `GetBytes`, macOS requires this explicit synchronization.
2. **Brace Hygiene**: During refactoring, ensure that `Release()` is called regardless of whether `StartAccess` succeeded, and that `EndAccess` is paired correctly to avoid deadlocks in the DeckLink driver.
3. **Green Screen Signature**: In UYVY or YUV formats, a buffer filled with `0x00` results in a bright green screen. If you see pure green, the first suspect is failed buffer mapping.

### Referenced Implementation
- [TestPatternSource.cpp](file:///Users/bongpark/Library/CloudStorage/OneDrive-한국기독교장로회총회유지재단/0.박봉환개인문서폴더/앱개발/Monitor3G/src/sources/TestPatternSource.cpp)
- [DeckLinkOutput.cpp](file:///Users/bongpark/Library/CloudStorage/OneDrive-한국기독교장로회총회유지재단/0.박봉환개인문서폴더/앱개발/Monitor3G/src/core/DeckLinkOutput.cpp)
