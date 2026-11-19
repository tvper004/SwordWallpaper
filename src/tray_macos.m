#import <Cocoa/Cocoa.h>

typedef void (*TrayCallback)(int action);

static TrayCallback g_tray_callback = NULL;
static NSStatusItem *g_status_item = nil;
static NSMenuItem *g_item_autostart = nil;
static NSMenuItem *g_item_hud = nil;
static NSMenuItem *g_item_show = nil;
static NSMenuItem *g_item_close = nil;

@interface AuraTrayDelegate : NSObject <NSApplicationDelegate>
- (void)onToggleAutostart:(id)sender;
- (void)onOpenHud:(id)sender;
- (void)onShowWindow:(id)sender;
- (void)onQuit:(id)sender;
- (BOOL)applicationShouldTerminateAfterLastWindowClosed:(NSApplication *)sender;
- (NSApplicationTerminateReply)applicationShouldTerminate:
    (NSApplication *)sender;
- (BOOL)applicationShouldHandleReopen:(NSApplication *)sender
                    hasVisibleWindows:(BOOL)flag;
@end

static AuraTrayDelegate *g_delegate = nil;
static id<NSApplicationDelegate> g_original_delegate = nil;
static BOOL g_is_quitting = NO;

static void run_on_main(dispatch_block_t block) {
  if ([NSThread isMainThread]) {
    block();
  } else {
    dispatch_async(dispatch_get_main_queue(), block);
  }
}

static NSWindow *g_main_window = nil;
static NSWindow *g_switcher_window = nil;

static BOOL is_main_settings_window(NSWindow *win) {
    if (!win || [win isKindOfClass:NSClassFromString(@"NSStatusBarWindow")]) {
        return NO;
    }
    if (g_main_window == win) {
        return YES;
    }
    if (g_switcher_window == win) {
        return NO;
    }
    // Switcher opens at full screen width (>= screen width); main window opens at ~1340x840
    NSScreen *scr = win.screen ?: [NSScreen mainScreen];
    if (scr && win.frame.size.width >= scr.frame.size.width - 2.0 && win.frame.size.height >= scr.frame.size.height - 2.0) {
        return NO;
    }
    return YES;
}

void hide_mac_main_window(void) {
    run_on_main(^{
        if ([NSApp delegate] != g_delegate) {
            if ([NSApp delegate] != nil) {
                g_original_delegate = [NSApp delegate];
            }
            [NSApp setDelegate:g_delegate];
        }
        if (g_item_show) {
            [g_item_show setTitle:@"Mostrar ventana"];
        }
        if (g_main_window) {
            [g_main_window orderOut:nil];
        } else {
            for (NSWindow *win in [NSApp windows]) {
                if (is_main_settings_window(win)) {
                    g_main_window = win;
                    [win orderOut:nil];
                }
            }
        }
    });
}

void show_mac_main_window(void) {
    run_on_main(^{
        [NSApp setActivationPolicy:NSApplicationActivationPolicyRegular];
        [NSApp activateIgnoringOtherApps:YES];
        if (g_item_show) {
            [g_item_show setTitle:@"Cerrar ventana"];
        }
        if (g_main_window) {
            [g_main_window setOpaque:NO];
            [g_main_window setBackgroundColor:[NSColor clearColor]];
            [g_main_window makeKeyAndOrderFront:nil];
        }
    });
}

void hide_mac_switcher_window(void) {
    run_on_main(^{
        if ([NSApp delegate] != g_delegate) {
            if ([NSApp delegate] != nil) {
                g_original_delegate = [NSApp delegate];
            }
            [NSApp setDelegate:g_delegate];
        }
        if (g_item_show) {
            [g_item_show setTitle:@"Mostrar ventana"];
        }
        if (g_switcher_window) {
            [g_switcher_window orderOut:nil];
        } else {
            for (NSWindow *win in [NSApp windows]) {
                if (![win isKindOfClass:NSClassFromString(@"NSStatusBarWindow")] && !is_main_settings_window(win)) {
                    g_switcher_window = win;
                    [win orderOut:nil];
                }
            }
        }
    });
}

void show_mac_switcher_window(void) {
    run_on_main(^{
        [NSApp setActivationPolicy:NSApplicationActivationPolicyRegular];
        [NSApp activateIgnoringOtherApps:YES];
        if (g_item_show) {
            [g_item_show setTitle:@"Cerrar ventana"];
        }
        if (g_switcher_window) {
            [g_switcher_window setOpaque:NO];
            [g_switcher_window setBackgroundColor:[NSColor clearColor]];
            [g_switcher_window makeKeyAndOrderFront:nil];
        }
    });
}

void zoom_mac_main_window(void) {
  run_on_main(^{
    if (g_main_window) {
      [g_main_window zoom:nil];
      return;
    }
    for (NSWindow *win in [NSApp windows]) {
      if (!is_main_settings_window(win)) {
        continue;
      }
      g_main_window = win;
      [win zoom:nil];
      break;
    }
  });
}

void macos_set_window_transparent(void) {
  run_on_main(^{
    for (NSWindow *win in [NSApp windows]) {
      if ([win isKindOfClass:NSClassFromString(@"NSStatusBarWindow")]) {
        continue;
      }
      if (is_main_settings_window(win)) {
        g_main_window = win;
      } else {
        g_switcher_window = win;
      }
      [win setOpaque:NO];
      [win setBackgroundColor:[NSColor clearColor]];
    }
  });
}

void minimize_mac_main_window(void) {
  run_on_main(^{
    if (g_main_window) {
      [g_main_window miniaturize:nil];
      return;
    }
    for (NSWindow *win in [NSApp windows]) {
      if (!is_main_settings_window(win)) {
        continue;
      }
      g_main_window = win;
      [win miniaturize:nil];
      break;
    }
  });
}

void ensure_mac_window_capabilities(void) {
  run_on_main(^{
    for (NSWindow *win in [NSApp windows]) {
      if ([win isKindOfClass:NSClassFromString(@"NSStatusBarWindow")]) {
        continue;
      }
      if (is_main_settings_window(win)) {
        g_main_window = win;
      } else {
        g_switcher_window = win;
      }
      [win setOpaque:NO];
      [win setBackgroundColor:[NSColor clearColor]];
    }
  });
}

void activate_mac_app(void) {
  run_on_main(^{
    [NSApp setActivationPolicy:NSApplicationActivationPolicyRegular];
    [NSApp activateIgnoringOtherApps:YES];
    ensure_mac_window_capabilities();
  });
}

void macos_get_screen_size(float *width, float *height) {
  @autoreleasepool {
    NSRect frame = [[NSScreen mainScreen] frame];
    if (width)
      *width = (float)frame.size.width;
    if (height)
      *height = (float)frame.size.height;
  }
}

int sword_wallpaper_macos_is_dark_appearance(void) {
  @autoreleasepool {
    NSString *style = [[NSUserDefaults standardUserDefaults]
        stringForKey:@"AppleInterfaceStyle"];
    return style != nil &&
           [style caseInsensitiveCompare:@"Dark"] == NSOrderedSame;
  }
}

@implementation AuraTrayDelegate

- (BOOL)respondsToSelector:(SEL)aSelector {
  if ([super respondsToSelector:aSelector]) {
    return YES;
  }
  return [g_original_delegate respondsToSelector:aSelector];
}

- (id)forwardingTargetForSelector:(SEL)aSelector {
  if ([g_original_delegate respondsToSelector:aSelector]) {
    return g_original_delegate;
  }
  return [super forwardingTargetForSelector:aSelector];
}

- (void)onToggleAutostart:(id)sender {
  if (g_tray_callback) {
    g_tray_callback(1); // 1 = ToggleAutostart
  }
}

- (void)onOpenHud:(id)sender {
  if (g_tray_callback) {
    g_tray_callback(5); // 5 = OpenSwitcher
  }
}

- (void)onShowWindow:(id)sender {
  BOOL any_visible = NO;
  for (NSWindow *win in [NSApp windows]) {
    if ([win isKindOfClass:NSClassFromString(@"NSStatusBarWindow")]) {
      continue;
    }
    if ([win isVisible]) {
      any_visible = YES;
      break;
    }
  }

  if (any_visible) {
    hide_mac_main_window();
    hide_mac_switcher_window();
    if (g_tray_callback) {
      g_tray_callback(4); // 4 = HideApp
    }
  } else {
    if (g_tray_callback) {
      g_tray_callback(2); // 2 = ShowApp
    }
  }
}

- (void)onQuit:(id)sender {
  g_is_quitting = YES;
  if (g_tray_callback) {
    g_tray_callback(3); // 3 = Quit
  }
}

- (BOOL)applicationShouldTerminateAfterLastWindowClosed:
    (NSApplication *)sender {
  return NO;
}

- (NSApplicationTerminateReply)applicationShouldTerminate:(NSApplication *)sender {
    if (!g_is_quitting) {
        NSAppleEventDescriptor *appleEvent = [[NSAppleEventManager sharedAppleEventManager] currentAppleEvent];
        NSEvent *currEvent = [NSApp currentEvent];
        BOOL isCmdQ = (currEvent && currEvent.type == NSEventTypeKeyDown
            && (currEvent.modifierFlags & NSEventModifierFlagCommand)
            && [[currEvent.charactersIgnoringModifiers lowercaseString] isEqualToString:@"q"]);
        BOOL isAuthorizedAppleEventQuit = NO;
        if (appleEvent && [appleEvent eventID] == kAEQuitApplication) {
            NSAppleEventDescriptor *reasonDesc = [appleEvent paramDescriptorForKeyword:kAEQuitReason];
            OSType reason = reasonDesc ? [reasonDesc typeCodeValue] : 0;
            if (reason == kAELogOut || reason == kAEReallyLogOut ||
                reason == kAEShowRestartDialog || reason == kAEShowShutdownDialog) {
                isAuthorizedAppleEventQuit = YES;
            }
            NSAppleEventDescriptor *pidDesc = [appleEvent attributeDescriptorForKeyword:keySenderPIDAttr];
            pid_t senderPid = pidDesc ? (pid_t)[pidDesc int32Value] : -1;
            if (senderPid > 0) {
                NSRunningApplication *senderApp =
                    [NSRunningApplication runningApplicationWithProcessIdentifier:senderPid];
                NSString *bundleId = senderApp.bundleIdentifier;
                if ([bundleId isEqualToString:@"com.apple.dock"] ||
                    [bundleId isEqualToString:@"com.apple.loginwindow"]) {
                    isAuthorizedAppleEventQuit = YES;
                }
            }
        }
        if (!isAuthorizedAppleEventQuit && !isCmdQ) {
            return NSTerminateCancel;
        }
        g_is_quitting = YES;
    }
    if (g_tray_callback) {
        g_tray_callback(3); // 3 = Quit
    }
    dispatch_after(dispatch_time(DISPATCH_TIME_NOW, (int64_t)(400 * NSEC_PER_MSEC)),
                   dispatch_get_main_queue(), ^{
        [NSApp replyToApplicationShouldTerminate:YES];
    });
    return NSTerminateLater;
}

- (BOOL)applicationShouldHandleReopen:(NSApplication *)sender
                    hasVisibleWindows:(BOOL)flag {
  if (g_tray_callback) {
    g_tray_callback(2); // ShowWindow
  }
  return YES;
}

@end

void setup_mac_tray(const uint8_t *icon_bytes, size_t icon_len,
                    bool autostart_initial, const char *title_autostart,
                    const char *title_hud,
                    const char *title_show, const char *title_close,
                    TrayCallback callback) {
  g_tray_callback = callback;

  NSData *data = nil;
  if (icon_bytes && icon_len > 0) {
    data = [NSData dataWithBytes:icon_bytes length:icon_len];
  }
  NSString *ns_autostart = title_autostart
                               ? [NSString stringWithUTF8String:title_autostart]
                               : @"Iniciar automáticamente";
  NSString *ns_hud = title_hud ? [NSString stringWithUTF8String:title_hud]
                               : @"Visualizar HUD";
  NSString *ns_show = title_show ? [NSString stringWithUTF8String:title_show]
                                 : @"Mostrar ventana";
  NSString *ns_close =
      title_close ? [NSString stringWithUTF8String:title_close] : @"Cerrar";

  run_on_main(^{
    [NSApplication sharedApplication];
    [[NSProcessInfo processInfo]
        disableAutomaticTermination:@"Background wallpaper service active"];
    [[NSProcessInfo processInfo] disableSuddenTermination];

    if (g_status_item != nil) {
      return;
    }

    if ([NSApp delegate] != nil && [NSApp delegate] != g_delegate) {
      g_original_delegate = [NSApp delegate];
    }
    g_delegate = [[AuraTrayDelegate alloc] init];
    [NSApp setDelegate:g_delegate];
    g_status_item = [[NSStatusBar systemStatusBar]
        statusItemWithLength:NSSquareStatusItemLength];

        // Track and style windows that appear
        [[NSNotificationCenter defaultCenter] addObserverForName:NSWindowDidBecomeKeyNotification
                                                          object:nil
                                                           queue:[NSOperationQueue mainQueue]
                                                      usingBlock:^(NSNotification * _Nonnull note) {
            NSWindow *win = (NSWindow *)note.object;
            if (win && ![win isKindOfClass:NSClassFromString(@"NSStatusBarWindow")]) {
                if (is_main_settings_window(win)) {
                    g_main_window = win;
                } else {
                    g_switcher_window = win;
                }
                [win setOpaque:NO];
                [win setBackgroundColor:[NSColor clearColor]];
                if (g_item_show) {
                    [g_item_show setTitle:@"Cerrar ventana"];
                }
            }
        }];
        [[NSNotificationCenter defaultCenter] addObserverForName:NSWindowDidBecomeMainNotification
                                                          object:nil
                                                           queue:[NSOperationQueue mainQueue]
                                                      usingBlock:^(NSNotification * _Nonnull note) {
            NSWindow *win = (NSWindow *)note.object;
            if (win && ![win isKindOfClass:NSClassFromString(@"NSStatusBarWindow")]) {
                if (is_main_settings_window(win)) {
                    g_main_window = win;
                } else {
                    g_switcher_window = win;
                }
                [win setOpaque:NO];
                [win setBackgroundColor:[NSColor clearColor]];
                if (g_item_show) {
                    [g_item_show setTitle:@"Cerrar ventana"];
                }
            }
        }];
        [[NSNotificationCenter defaultCenter] addObserverForName:NSWindowWillCloseNotification
                                                          object:nil
                                                           queue:[NSOperationQueue mainQueue]
                                                      usingBlock:^(NSNotification * _Nonnull note) {
            NSWindow *win = (NSWindow *)note.object;
            if (win && ![win isKindOfClass:NSClassFromString(@"NSStatusBarWindow")]) {
                if (g_item_show) {
                    [g_item_show setTitle:@"Mostrar ventana"];
                }
            }
        }];
        ensure_mac_window_capabilities();

    if (data) {
      NSImage *image = [[NSImage alloc] initWithData:data];
      if (image) {
        [image setSize:NSMakeSize(20, 20)];
        [image setTemplate:YES];
        g_status_item.button.image = image;
        g_status_item.button.imagePosition = NSImageOnly;
      }
    }
    if (g_status_item.button.image == nil) {
      g_status_item.button.title = @"⚔ Sword Wallpaper";
    }
    g_status_item.button.toolTip = @"Sword Wallpaper";

    NSMenu *menu = [[NSMenu alloc] initWithTitle:@"Sword Wallpaper"];

    // 1. Iniciar automáticamente
    g_item_autostart =
        [[NSMenuItem alloc] initWithTitle:ns_autostart
                                   action:@selector(onToggleAutostart:)
                            keyEquivalent:@""];
    [g_item_autostart setTarget:g_delegate];
    [g_item_autostart setState:autostart_initial ? NSControlStateValueOn
                                                 : NSControlStateValueOff];
    [menu addItem:g_item_autostart];

    // 2. Visualizar HUD
    g_item_hud =
        [[NSMenuItem alloc] initWithTitle:ns_hud
                                   action:@selector(onOpenHud:)
                            keyEquivalent:@""];
    [g_item_hud setTarget:g_delegate];
    [menu addItem:g_item_hud];

    BOOL any_visible = NO;
    for (NSWindow *win in [NSApp windows]) {
      if ([win isKindOfClass:NSClassFromString(@"NSStatusBarWindow")]) {
        continue;
      }
      if ([win isVisible]) {
        any_visible = YES;
        if (is_main_settings_window(win)) {
          g_main_window = win;
        } else {
          g_switcher_window = win;
        }
        break;
      }
    }
    NSString *initial_show_title = any_visible ? @"Cerrar ventana" : ns_show;

    // 3. Mostrar ventana / Cerrar ventana
    g_item_show = [[NSMenuItem alloc] initWithTitle:initial_show_title
                                             action:@selector(onShowWindow:)
                                      keyEquivalent:@""];
    [g_item_show setTarget:g_delegate];
    [menu addItem:g_item_show];

    [menu addItem:[NSMenuItem separatorItem]];

    // 4. Cerrar
    g_item_close = [[NSMenuItem alloc] initWithTitle:ns_close
                                              action:@selector(onQuit:)
                                       keyEquivalent:@"q"];
    [g_item_close setTarget:g_delegate];
    [menu addItem:g_item_close];

    g_status_item.menu = menu;

    // Listen for system shutdown / logout to cleanly quit without blocking
    // macOS shutdown
    [[[NSWorkspace sharedWorkspace] notificationCenter]
        addObserverForName:NSWorkspaceWillPowerOffNotification
                    object:nil
                     queue:[NSOperationQueue mainQueue]
                usingBlock:^(NSNotification *_Nonnull __unused note) {
                  g_is_quitting = YES;
                  if (g_tray_callback) {
                    g_tray_callback(3); // QuitApp
                  }
                }];
  });
}

void update_mac_tray_autostart(bool autostart) {
  run_on_main(^{
    if (g_item_autostart) {
      [g_item_autostart
          setState:autostart ? NSControlStateValueOn : NSControlStateValueOff];
    }
  });
}

void update_mac_tray_labels(const char *title_autostart, const char *title_hud,
                            const char *title_show, const char *title_close) {
  NSString *ns_autostart =
      title_autostart ? [NSString stringWithUTF8String:title_autostart] : nil;
  NSString *ns_hud =
      title_hud ? [NSString stringWithUTF8String:title_hud] : nil;
  NSString *ns_show =
      title_show ? [NSString stringWithUTF8String:title_show] : nil;
  NSString *ns_close =
      title_close ? [NSString stringWithUTF8String:title_close] : nil;

  run_on_main(^{
    if (ns_autostart && g_item_autostart) {
      [g_item_autostart setTitle:ns_autostart];
    }
    if (ns_hud && g_item_hud) {
      [g_item_hud setTitle:ns_hud];
    }
    if (ns_show && g_item_show) {
      [g_item_show setTitle:ns_show];
    }
    if (ns_close && g_item_close) {
      [g_item_close setTitle:ns_close];
    }
  });
}

void update_mac_tray_show_label(const char *title_show) {
  NSString *ns_show =
      title_show ? [NSString stringWithUTF8String:title_show] : nil;
  run_on_main(^{
    if (ns_show && g_item_show) {
      [g_item_show setTitle:ns_show];
    }
  });
}
