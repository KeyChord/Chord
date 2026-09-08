// Run with:
// clang -Wall -Wextra -Werror tests/hid_listener.c -framework IOKit -framework CoreFoundation -o /tmp/chord-hid-test && /tmp/chord-hid-test
#include <assert.h>
#include "../native/hid.c"

static int notifications[8];
static int notification_count;

static void readiness(int value) {
    assert(notification_count < 8);
    notifications[notification_count++] = value;
}

int main(void) {
    listener_state state = {
        .callback = readiness,
        .opened_devices = CFSetCreateMutable(NULL, 0, &kCFTypeSetCallBacks),
    };
    // The callbacks only retain device identities; CF strings stand in for devices.
    IOHIDDeviceRef physical = (IOHIDDeviceRef)CFSTR("physical");
    IOHIDDeviceRef virtual_keyboard = (IOHIDDeviceRef)CFSTR("virtual");
    IOHIDDeviceRef external = (IOHIDDeviceRef)CFSTR("external");

    device_matched(&state, kIOReturnExclusiveAccess, NULL, physical);
    assert(notification_count == 0);
    device_matched(&state, kIOReturnSuccess, NULL, virtual_keyboard);
    assert(notification_count == 1 && notifications[0] == -1);
    device_matched(&state, kIOReturnSuccess, NULL, virtual_keyboard);
    device_matched(&state, kIOReturnSuccess, NULL, external);
    device_removed(&state, kIOReturnSuccess, NULL, physical);
    device_removed(&state, kIOReturnSuccess, NULL, virtual_keyboard);
    assert(notification_count == 1);
    device_removed(&state, kIOReturnSuccess, NULL, external);
    assert(notification_count == 2 && notifications[1] == -2);
    device_removed(&state, kIOReturnSuccess, NULL, external);
    device_matched(&state, kIOReturnNotPermitted, NULL, physical);
    assert(notification_count == 2);
    device_matched(&state, kIOReturnSuccess, NULL, virtual_keyboard);
    assert(notification_count == 3 && notifications[2] == -1);

    input_callback(NULL, kIOReturnError, NULL, NULL);
    CFRelease(state.opened_devices);
    puts("HID listener readiness tests passed");
}
