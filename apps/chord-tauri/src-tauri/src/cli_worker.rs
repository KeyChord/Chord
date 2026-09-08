//! Keep the CLI's main thread available to native add-ons while Bun runs on its worker.

use std::future::Future;

pub fn run<F, Fut>(task: F) -> anyhow::Result<()>
where
    F: FnOnce() -> Fut + Send + 'static,
    Fut: Future<Output = anyhow::Result<()>>,
{
    // Install a source before starting the worker: an empty macOS run loop returns
    // immediately instead of waiting for main-queue work.
    #[cfg(target_os = "macos")]
    let main_loop = MainLoop::new();

    let worker = std::thread::spawn(move || {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()?;
        runtime.block_on(task())
    });

    #[cfg(target_os = "macos")]
    while !worker.is_finished() {
        main_loop.tick();
    }

    // Joining only after completion on macOS avoids starving Swift's MainActor.
    // Other platforms retain their existing blocking wait.
    worker
        .join()
        .unwrap_or_else(|panic| std::panic::resume_unwind(panic))
}

#[cfg(target_os = "macos")]
struct MainLoop {
    run_loop: objc2::rc::Retained<objc2_foundation::NSRunLoop>,
    port: objc2::rc::Retained<objc2_foundation::NSMachPort>,
}

#[cfg(target_os = "macos")]
impl MainLoop {
    fn new() -> Self {
        use objc2_foundation::{NSDefaultRunLoopMode, NSMachPort, NSRunLoop};

        assert!(
            objc2::MainThreadMarker::new().is_some(),
            "CLI must start on the main thread"
        );
        let run_loop = NSRunLoop::mainRunLoop();
        let port = NSMachPort::new();
        // SAFETY: The port and run loop are owned and accessed on the main thread.
        unsafe { run_loop.addPort_forMode(&port, NSDefaultRunLoopMode) };
        Self { run_loop, port }
    }

    fn tick(&self) {
        objc2::rc::autoreleasepool(|_| {
            // Bound the wait so worker completion (including failure) exits promptly.
            let deadline = objc2_foundation::NSDate::dateWithTimeIntervalSinceNow(0.01);
            self.run_loop.runUntilDate(&deadline);
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
