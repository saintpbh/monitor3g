#import <AVFoundation/AVFoundation.h>
#import <CoreVideo/CoreVideo.h>
#import <Foundation/Foundation.h>

// Define callback signature
typedef void (*FrameCallback)(const uint8_t *data, size_t len, int width,
                              int height, size_t bytesPerRow, void *ctx);

@interface LuminaCameraDelegate
    : NSObject <AVCaptureVideoDataOutputSampleBufferDelegate> {
  FrameCallback _callback;
  void *_ctx;
}

- (instancetype)initWithCallback:(FrameCallback)callback context:(void *)ctx;

@end

@implementation LuminaCameraDelegate

- (instancetype)initWithCallback:(FrameCallback)callback context:(void *)ctx {
  self = [super init];
  if (self) {
    _callback = callback;
    _ctx = ctx;
  }
  return self;
}

- (void)captureOutput:(AVCaptureOutput *)output
    didOutputSampleBuffer:(CMSampleBufferRef)sampleBuffer
           fromConnection:(AVCaptureConnection *)connection {

  CVImageBufferRef imageBuffer = CMSampleBufferGetImageBuffer(sampleBuffer);
  if (!imageBuffer)
    return;

  // Lock the base address of the pixel buffer
  CVPixelBufferLockBaseAddress(imageBuffer, kCVPixelBufferLock_ReadOnly);

  const uint8_t *baseAddress =
      (const uint8_t *)CVPixelBufferGetBaseAddress(imageBuffer);
  size_t width = CVPixelBufferGetWidth(imageBuffer);
  size_t height = CVPixelBufferGetHeight(imageBuffer);
  size_t bytesPerRow = CVPixelBufferGetBytesPerRow(imageBuffer);
  size_t totalSize = CVPixelBufferGetDataSize(imageBuffer);

  if (_callback && baseAddress) {
    _callback(baseAddress, totalSize, (int)width, (int)height, bytesPerRow,
              _ctx);
  }

  CVPixelBufferUnlockBaseAddress(imageBuffer, kCVPixelBufferLock_ReadOnly);
}

@end

// Export C functions
void *create_lumina_delegate(FrameCallback callback, void *ctx) {
  LuminaCameraDelegate *delegate =
      [[LuminaCameraDelegate alloc] initWithCallback:callback context:ctx];
  return (__bridge_retained void *)delegate;
}

void release_lumina_delegate(void *ptr) {
  if (ptr) {
    LuminaCameraDelegate *delegate =
        (__bridge_transfer LuminaCameraDelegate *)ptr;
    (void)delegate; // Suppress unused variable warning
    delegate = nil;
  }
}
