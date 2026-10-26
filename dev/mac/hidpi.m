// A Retina screen for the hosted Mac, whose own is 1024 × 768 at 1×: a virtual display at
// 1280 × 800 points, 2× (2560 × 800·2 pixels), made the main display — where the menu bar
// is and windows open. It lasts as long as this process runs; it prints the display's id
// and its scale once it's in place. Built and run by dev/mac/scenarios/editor-2x.sh:
//
//   clang -fobjc-arc -framework Foundation -framework CoreGraphics dev/mac/hidpi.m -o hidpi
//
// CGVirtualDisplay is CoreGraphics' own, not in its headers: declared here as it answers.
#import <CoreGraphics/CoreGraphics.h>
#import <Foundation/Foundation.h>

@interface CGVirtualDisplayDescriptor : NSObject
@property(retain, nonatomic) dispatch_queue_t queue;
@property(retain, nonatomic) NSString *name;
@property(nonatomic) unsigned int maxPixelsWide;
@property(nonatomic) unsigned int maxPixelsHigh;
@property(nonatomic) CGSize sizeInMillimeters;
@property(nonatomic) unsigned int productID;
@property(nonatomic) unsigned int vendorID;
@property(nonatomic) unsigned int serialNum;
@property(copy, nonatomic) void (^terminationHandler)(id, id);
@end
@interface CGVirtualDisplayMode : NSObject
- (instancetype)initWithWidth:(unsigned int)width height:(unsigned int)height refreshRate:(double)refreshRate;
@end
@interface CGVirtualDisplaySettings : NSObject
@property(retain, nonatomic) NSArray *modes;
@property(nonatomic) unsigned int hiDPI;
@end
@interface CGVirtualDisplay : NSObject
- (instancetype)initWithDescriptor:(CGVirtualDisplayDescriptor *)descriptor;
- (BOOL)applySettings:(CGVirtualDisplaySettings *)settings;
@property(readonly, nonatomic) unsigned int displayID;
@end

static const unsigned W = 1280, H = 800;

int main(void) {
  @autoreleasepool {
    CGVirtualDisplayDescriptor *d = [CGVirtualDisplayDescriptor new];
    d.queue = dispatch_get_main_queue();
    d.name = @"shroom 2x";
    d.maxPixelsWide = W * 2;
    d.maxPixelsHigh = H * 2;
    d.sizeInMillimeters = CGSizeMake(286, 179);
    d.productID = 0x5352;
    d.vendorID = 0x5352;
    d.serialNum = 2;
    d.terminationHandler = ^(id a, id b) {
      fprintf(stderr, "hidpi: the display went away\n");
      exit(1);
    };
    CGVirtualDisplay *v = [[CGVirtualDisplay alloc] initWithDescriptor:d];
    if (!v) {
      fprintf(stderr, "hidpi: no virtual display on this Mac\n");
      return 1;
    }
    CGVirtualDisplaySettings *s = [CGVirtualDisplaySettings new];
    s.hiDPI = 1;
    s.modes = @[ [[CGVirtualDisplayMode alloc] initWithWidth:W * 2 height:H * 2 refreshRate:60],
                 [[CGVirtualDisplayMode alloc] initWithWidth:W height:H refreshRate:60] ];
    if (![v applySettings:s]) {
      fprintf(stderr, "hidpi: its settings were refused\n");
      return 1;
    }
    CGDirectDisplayID vid = v.displayID, was = CGMainDisplayID();
    [[NSRunLoop mainRunLoop] runUntilDate:[NSDate dateWithTimeIntervalSinceNow:2]];
    // Its 2× mode: W × H points over twice the pixels.
    CGDisplayModeRef pick = NULL;
    NSDictionary *opts = @{(__bridge NSString *)kCGDisplayShowDuplicateLowResolutionModes : @YES};
    CFArrayRef modes = CGDisplayCopyAllDisplayModes(vid, (__bridge CFDictionaryRef)opts);
    for (CFIndex i = 0; modes && i < CFArrayGetCount(modes); i++) {
      CGDisplayModeRef m = (CGDisplayModeRef)CFArrayGetValueAtIndex(modes, i);
      fprintf(stderr, "hidpi: mode %zu × %zu over %zu × %zu\n", CGDisplayModeGetWidth(m), CGDisplayModeGetHeight(m), CGDisplayModeGetPixelWidth(m), CGDisplayModeGetPixelHeight(m));
      if (CGDisplayModeGetWidth(m) == W && CGDisplayModeGetPixelWidth(m) == W * 2) pick = m;
    }
    CGDisplayConfigRef c;
    CGBeginDisplayConfiguration(&c);
    if (pick) CGConfigureDisplayWithDisplayMode(c, vid, pick, NULL);
    // The main display is the one at the origin; the Mac's own goes to its right.
    CGConfigureDisplayOrigin(c, vid, 0, 0);
    CGConfigureDisplayOrigin(c, was, (int32_t)W, 0);
    CGError e = CGCompleteDisplayConfiguration(c, kCGConfigureForSession);
    if (modes) CFRelease(modes);
    [[NSRunLoop mainRunLoop] runUntilDate:[NSDate dateWithTimeIntervalSinceNow:2]];
    CGDisplayModeRef now = CGDisplayCopyDisplayMode(vid);
    printf("display %u: %zu × %zu points over %zu × %zu pixels, main %s (configure %d)\n", vid, CGDisplayModeGetWidth(now), CGDisplayModeGetHeight(now),
           CGDisplayModeGetPixelWidth(now), CGDisplayModeGetPixelHeight(now), CGMainDisplayID() == vid ? "yes" : "no", e);
    fflush(stdout);
    CGDisplayModeRelease(now);
    [[NSRunLoop mainRunLoop] run];
  }
  return 0;
}
