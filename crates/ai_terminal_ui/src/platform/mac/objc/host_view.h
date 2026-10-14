#import <AppKit/AppKit.h>
#import <QuartzCore/QuartzCore.h>

@interface NSPasteboard (AiTerminal)
- (NSArray *)getFilePaths;
@end

/// AiTerminalHostView is the Content view of an AI Terminal window.
// It is backed by a Metal CALayer.
@interface AiTerminalHostView : NSView <CALayerDelegate, NSTextInputClient>
- (AiTerminalHostView *)initWithFrame:(NSRect)frame
                          metalDevice:(id)metalDevice
                   enableTitlebarDrag:(BOOL)enableTitlebarDrag
                             testMode:(BOOL)testMode;
- (void)setAsyncCallback:(BOOL)shouldAsync;
- (void)setPresentsWithTransaction:(BOOL)presentsWithTransaction;
- (BOOL)keyDownImpl:(NSEvent *)event;
@end
