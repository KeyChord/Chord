//! Keep the CLI's main thread available to native add-ons while Bun runs on its worker.

use std::future::Future;

pub fn run<F, Fut>(task: F) -> anyhow::Result<()>
where
    F: FnOnce() -> Fut + Send + 'static,
    Fut: Future<Output = anyhow::Result<()>>,
{
    run_with_ui(task, false)
}

pub fn run_package<F, Fut>(task: F) -> anyhow::Result<()>
where
    F: FnOnce() -> Fut + Send + 'static,
    Fut: Future<Output = anyhow::Result<()>>,
{
    run_with_ui(task, true)
}

fn run_with_ui<F, Fut>(task: F, package_ui: bool) -> anyhow::Result<()>
where
    F: FnOnce() -> Fut + Send + 'static,
    Fut: Future<Output = anyhow::Result<()>>,
{
    // Install a source before starting the worker: an empty macOS run loop returns
    // immediately instead of waiting for main-queue work.
    #[cfg(target_os = "macos")]
    let main_loop = MainLoop::new(package_ui);

    let finished = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let worker = std::thread::spawn({
        let finished = finished.clone();
        move || {
            // Also on a panic: the main thread must never be left waiting.
            let _finished = Finished {
                flag: finished,
                package_ui,
            };
            let mut builder = if package_ui {
                tokio::runtime::Builder::new_current_thread()
            } else {
                tokio::runtime::Builder::new_multi_thread()
            };
            let runtime = builder.enable_all().build()?;
            runtime.block_on(task())
        }
    });

    #[cfg(target_os = "macos")]
    while !finished.load(std::sync::atomic::Ordering::Acquire) {
        main_loop.tick();
    }

    // Joining only after completion on macOS avoids starving Swift's MainActor.
    // Other platforms retain their existing blocking wait.
    let result = worker.join();
    chords_lib::native_input::release_standalone_mouse();
    result.unwrap_or_else(|panic| std::panic::resume_unwind(panic))
}

/// Marks the worker thread as done when it ends, however it ends.
struct Finished {
    flag: std::sync::Arc<std::sync::atomic::AtomicBool>,
    package_ui: bool,
}

impl Drop for Finished {
    fn drop(&mut self) {
        self.flag.store(true, std::sync::atomic::Ordering::Release);
        #[cfg(target_os = "macos")]
        if self.package_ui {
            wake_application();
        }
    }
}

/// Makes the package worker's main thread, which waits for the next event with no
/// time limit, return and notice that the worker thread is done.
#[cfg(target_os = "macos")]
fn wake_application() {
    use objc2::runtime::AnyObject;
    use objc2::{class, msg_send};
    use objc2_app_kit::{NSEvent, NSEventModifierFlags, NSEventType};
    use objc2_foundation::NSPoint;

    // SAFETY: an event may be created on any thread.
    let event = unsafe {
        NSEvent::otherEventWithType_location_modifierFlags_timestamp_windowNumber_context_subtype_data1_data2(
            NSEventType::ApplicationDefined,
            NSPoint::new(0.0, 0.0),
            NSEventModifierFlags::empty(),
            0.0,
            0,
            None,
            0,
            0,
            0,
        )
    };
    let Some(event) = event else {
        return;
    };
    // SAFETY: the application object exists (`MainLoop::new` made it), and AppKit
    // documents `postEvent:atStart:` as callable from other threads.
    unsafe {
        let application: *mut AnyObject = msg_send![class!(NSApplication), sharedApplication];
        let _: () = msg_send![application, postEvent: &*event, atStart: false];
    }
}

#[cfg(target_os = "macos")]
struct MainLoop {
    run_loop: objc2::rc::Retained<objc2_foundation::NSRunLoop>,
    port: objc2::rc::Retained<objc2_foundation::NSMachPort>,
    application: Option<objc2::rc::Retained<objc2_app_kit::NSApplication>>,
}

#[cfg(target_os = "macos")]
impl MainLoop {
    fn new(package_ui: bool) -> Self {
        use objc2_foundation::{NSDefaultRunLoopMode, NSMachPort, NSRunLoop};

        assert!(
            objc2::MainThreadMarker::new().is_some(),
            "CLI must start on the main thread"
        );
        let run_loop = NSRunLoop::mainRunLoop();
        let port = NSMachPort::new();
        // SAFETY: The port and run loop are owned and accessed on the main thread.
        unsafe { run_loop.addPort_forMode(&port, NSDefaultRunLoopMode) };
        let application = package_ui.then(|| {
            use objc2_app_kit::{NSApplication, NSApplicationActivationPolicy};
            let application = NSApplication::sharedApplication(
                objc2::MainThreadMarker::new().expect("package UI must run on main thread"),
            );
            application.setActivationPolicy(NSApplicationActivationPolicy::Accessory);
            application.finishLaunching();
            application
        });
        Self {
            run_loop,
            port,
            application,
        }
    }

    fn tick(&self) {
        objc2::rc::autoreleasepool(|_| {
            // A package worker lives as long as the app and sits idle most of that
            // time, so it waits for the next event with no time limit; the worker
            // thread posts one when it ends (see `Finished`). A CLI run is short and
            // bounds the wait instead.
            let deadline = if self.application.is_some() {
                objc2_foundation::NSDate::distantFuture()
            } else {
                objc2_foundation::NSDate::dateWithTimeIntervalSinceNow(0.01)
            };
            if let Some(application) = &self.application {
                let event = application.nextEventMatchingMask_untilDate_inMode_dequeue(
                    objc2_app_kit::NSEventMask::Any,
                    Some(&deadline),
                    // SAFETY: Foundation owns this immutable process-lifetime run-loop mode.
                    unsafe { objc2_foundation::NSDefaultRunLoopMode },
                    true,
                );
                if let Some(event) = event {
                    application.sendEvent(&event);
                }
                application.updateWindows();
            } else {
                self.run_loop.runUntilDate(&deadline);
            }
        });
    }
}

#[cfg(target_os = "macos")]
impl Drop for MainLoop {
    fn drop(&mut self) {
        // SAFETY: Remove the same source on the thread where it was installed.
        unsafe {
            self.run_loop
                .removePort_forMode(&self.port, objc2_foundation::NSDefaultRunLoopMode);
        }
        self.port.invalidate();
    }
}
