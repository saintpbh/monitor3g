/// LuminaTV Virtual Display — Objective-C bridge for CGVirtualDisplay
/// Creates a macOS virtual monitor that applications (PowerPoint, Keynote)
/// can target for slide show output. LuminaTV then captures this display
/// via ScreenCaptureKit with zero-copy.
///
/// API surface discovered via runtime class-dump:
///   CGVirtualDisplay: initWithDescriptor:, applySettings:, displayID
///   CGVirtualDisplayDescriptor: name, maxPixelsWide/High, sizeInMillimeters,
///   vendorID, productID, serialNum, queue (dispatch) CGVirtualDisplaySettings:
///   modes, hiDPI CGVirtualDisplayMode: initWithWidth:height:refreshRate:

#import <CoreGraphics/CoreGraphics.h>
#import <Foundation/Foundation.h>
#import <objc/runtime.h>

// ========== CGVirtualDisplay Private API Declarations ==========

@interface CGVirtualDisplayMode : NSObject
- (instancetype)initWithWidth:(NSUInteger)width
                       height:(NSUInteger)height
                  refreshRate:(double)refreshRate;
@property(readonly) NSUInteger width;
@property(readonly) NSUInteger height;
@property(readonly) double refreshRate;
@end

@interface CGVirtualDisplayDescriptor : NSObject
@property(nonatomic, copy) NSString *name;
@property(nonatomic) NSUInteger maxPixelsWide;
@property(nonatomic) NSUInteger maxPixelsHigh;
@property(nonatomic) CGSize sizeInMillimeters;
@property(nonatomic) uint32_t vendorID;
@property(nonatomic) uint32_t productID;
@property(nonatomic) uint32_t serialNum;
@property(nonatomic, retain) dispatch_queue_t dispatchQueue;
@property(nonatomic, copy) void (^terminationHandler)(id display, id error);
@end

@interface CGVirtualDisplaySettings : NSObject
@property(nonatomic, copy) NSArray<CGVirtualDisplayMode *> *modes;
@property(nonatomic) BOOL hiDPI;
@end

@interface CGVirtualDisplay : NSObject
- (instancetype)initWithDescriptor:(CGVirtualDisplayDescriptor *)descriptor;
- (CGDirectDisplayID)displayID;
- (BOOL)applySettings:(CGVirtualDisplaySettings *)settings;
@end

// ========== C API for Rust FFI ==========

void *lumina_create_virtual_display(uint32_t width, uint32_t height,
                                    const char *name) {
  // Verify classes exist (macOS 14+ check)
  Class descClass = NSClassFromString(@"CGVirtualDisplayDescriptor");
  Class modeClass = NSClassFromString(@"CGVirtualDisplayMode");
  Class displayClass = NSClassFromString(@"CGVirtualDisplay");
  Class settingsClass = NSClassFromString(@"CGVirtualDisplaySettings");

  if (!descClass || !modeClass || !displayClass || !settingsClass) {
    NSLog(@"[VirtualDisplay] CGVirtualDisplay API not available (requires "
          @"macOS 14+)");
    return NULL;
  }

  @try {
    // 1. Create descriptor
    CGVirtualDisplayDescriptor *desc = [[descClass alloc] init];
    desc.name = [NSString stringWithUTF8String:name];
    desc.maxPixelsWide = width;
    desc.maxPixelsHigh = height;
    // ~24" display physical size (matches typical presentation monitor)
    desc.sizeInMillimeters = CGSizeMake(527, 296);
    desc.vendorID = 0x1234;  // Custom vendor ID
    desc.productID = 0x0001; // LuminaTV product
    desc.serialNum = 1;
    desc.dispatchQueue =
        dispatch_get_global_queue(DISPATCH_QUEUE_PRIORITY_HIGH, 0);

    NSLog(@"[VirtualDisplay] Descriptor: name='%s' %ux%u", name, width, height);

    // 2. Create the virtual display from descriptor
    CGVirtualDisplay *display = [[displayClass alloc] initWithDescriptor:desc];
    if (!display) {
      NSLog(
          @"[VirtualDisplay] Failed to create virtual display from descriptor");
      return NULL;
    }

    CGDirectDisplayID displayID = [display displayID];
    NSLog(@"[VirtualDisplay] Display created with ID: %u (0x%x)", displayID,
          displayID);

    if (displayID == 0) {
      NSLog(@"[VirtualDisplay] Display ID is 0 — creation failed silently");
      return NULL;
    }

    // 3. Create display mode
    CGVirtualDisplayMode *mode = [[modeClass alloc] initWithWidth:width
                                                           height:height
                                                      refreshRate:60.0];
    if (!mode) {
      NSLog(@"[VirtualDisplay] Failed to create display mode %ux%u@60Hz", width,
            height);
      return NULL;
    }

    NSLog(@"[VirtualDisplay] Mode: %lux%lu @ %.0fHz", (unsigned long)mode.width,
          (unsigned long)mode.height, mode.refreshRate);

    // 4. Create settings and apply modes to the display
    CGVirtualDisplaySettings *settings = [[settingsClass alloc] init];
    settings.modes = @[ mode ];
    settings.hiDPI = NO; // Standard DPI for presentation output

    BOOL applied = [display applySettings:settings];
    NSLog(@"[VirtualDisplay] Settings applied: %@", applied ? @"YES" : @"NO");

    if (!applied) {
      NSLog(@"[VirtualDisplay] WARNING: applySettings failed — display may not "
            @"be visible");
    }

    NSLog(@"[VirtualDisplay] ✅ Created '%s' (%ux%u @ 60Hz) → Display ID: %u",
          name, width, height, displayID);

    // Retain and return as opaque pointer
    return (__bridge_retained void *)display;

  } @catch (NSException *exception) {
    NSLog(@"[VirtualDisplay] Exception: %@", exception);
    return NULL;
  }
}

uint32_t lumina_get_virtual_display_id(void *display_ptr) {
  if (!display_ptr)
    return 0;

  CGVirtualDisplay *display = (__bridge CGVirtualDisplay *)display_ptr;
  return (uint32_t)[display displayID];
}

void lumina_destroy_virtual_display(void *display_ptr) {
  if (!display_ptr)
    return;

  CGVirtualDisplay *display = (__bridge_transfer CGVirtualDisplay *)display_ptr;
  CGDirectDisplayID displayID = [display displayID];
  NSLog(@"[VirtualDisplay] Destroyed virtual display (ID: %u)", displayID);
  display = nil; // ARC releases
}
