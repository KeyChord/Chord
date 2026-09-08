#include <IOKit/hid/IOHIDManager.h>
#include <CoreFoundation/CoreFoundation.h>
#include <stdio.h>

typedef void (*capslock_callback)(int pressed);

static capslock_callback rust_callback = NULL;

typedef struct {
    capslock_callback callback;
    CFMutableSetRef opened_devices;
} listener_state;

static void device_matched(void *context, IOReturn result, void *sender, IOHIDDeviceRef device) {
    (void)sender;
    listener_state *state = context;
    if (result != kIOReturnSuccess) {
        fprintf(stderr, "Caps Lock HID: skipping keyboard (0x%08x%s)\n",
                (unsigned int)result,
                result == kIOReturnExclusiveAccess ? ", exclusive access" : "");
        return;
    }
    bool was_ready = CFSetGetCount(state->opened_devices) > 0;
    CFSetAddValue(state->opened_devices, device);
    if (!was_ready) state->callback(-1);
}

static void device_removed(void *context, IOReturn result, void *sender, IOHIDDeviceRef device) {
    (void)result;
    (void)sender;
    listener_state *state = context;
    bool was_ready = CFSetGetCount(state->opened_devices) > 0;
    CFSetRemoveValue(state->opened_devices, device);
    if (was_ready && CFSetGetCount(state->opened_devices) == 0) state->callback(-2);
}

static void input_callback(
    void *context,
    IOReturn result,
    void *sender,
    IOHIDValueRef value
) {
    (void)context;
    (void)sender;

    if (result != kIOReturnSuccess || !value) return;

    IOHIDElementRef element = IOHIDValueGetElement(value);

    uint32_t usage_page = IOHIDElementGetUsagePage(element);
    uint32_t usage = IOHIDElementGetUsage(element);

    // keyboard page
    if (usage_page != 0x07)
        return;

    // caps lock
    if (usage != 0x39)
        return;

    int pressed = IOHIDValueGetIntegerValue(value);

    if (rust_callback) {
        rust_callback(pressed);
    }
}

int start_caps_lock_listener(capslock_callback cb) {
    IOHIDManagerRef manager =
        IOHIDManagerCreate(kCFAllocatorDefault, kIOHIDOptionsTypeNone);

    if (!manager) return kIOReturnNoMemory;

    // Match keyboard devices only
    CFMutableDictionaryRef matching =
        CFDictionaryCreateMutable(
            kCFAllocatorDefault,
            0,
            &kCFTypeDictionaryKeyCallBacks,
            &kCFTypeDictionaryValueCallBacks
        );

    int page = kHIDPage_GenericDesktop;
    int usage = kHIDUsage_GD_Keyboard;

    CFNumberRef page_ref =
        CFNumberCreate(kCFAllocatorDefault, kCFNumberIntType, &page);

    CFNumberRef usage_ref =
        CFNumberCreate(kCFAllocatorDefault, kCFNumberIntType, &usage);

    if (!matching || !page_ref || !usage_ref) {
        if (matching) CFRelease(matching);
        if (page_ref) CFRelease(page_ref);
        if (usage_ref) CFRelease(usage_ref);
        CFRelease(manager);
        return kIOReturnNoMemory;
    }

    CFDictionarySetValue(
        matching,
        CFSTR(kIOHIDDeviceUsagePageKey),
        page_ref
    );

    CFDictionarySetValue(
        matching,
        CFSTR(kIOHIDDeviceUsageKey),
        usage_ref
    );

    IOHIDManagerSetDeviceMatching(manager, matching);
    CFRelease(matching);
    CFRelease(page_ref);
    CFRelease(usage_ref);

    listener_state state = {
        .callback = cb,
        .opened_devices = CFSetCreateMutable(kCFAllocatorDefault, 0, &kCFTypeSetCallBacks),
    };
    if (!state.opened_devices) {
        CFRelease(manager);
        return kIOReturnNoMemory;
    }
    rust_callback = cb;
    IOHIDManagerRegisterDeviceMatchingCallback(manager, device_matched, &state);
    IOHIDManagerRegisterDeviceRemovalCallback(manager, device_removed, &state);

    IOHIDManagerRegisterInputValueCallback(
        manager,
        input_callback,
        NULL
    );

    IOHIDManagerScheduleWithRunLoop(
        manager,
        CFRunLoopGetCurrent(),
        kCFRunLoopDefaultMode
    );

    IOReturn result = IOHIDManagerOpen(manager, kIOHIDOptionsTypeNone);
    // The manager returns a device's failure even if other keyboards opened.
    // Karabiner seizes physical keyboards but exposes an accessible virtual one.
    // Readiness comes from successful per-device callbacks, including hot-plug.
    if (result == kIOReturnSuccess || result == kIOReturnExclusiveAccess || result == kIOReturnNoDevice) {
        CFRunLoopRun();
        result = kIOReturnSuccess;
    }
    IOHIDManagerClose(manager, kIOHIDOptionsTypeNone);
    IOHIDManagerUnscheduleFromRunLoop(manager, CFRunLoopGetCurrent(), kCFRunLoopDefaultMode);
    CFRelease(manager);
    CFRelease(state.opened_devices);
    cb(-2);
    rust_callback = NULL;
    return result;
}
