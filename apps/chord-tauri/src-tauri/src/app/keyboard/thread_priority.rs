//! Scheduling priority for the threads every keystroke waits on.
//!
//! The event tap is an active one: macOS holds each key until the tap thread has
//! answered, for every app. A Rust thread starts at the default priority, where it
//! queues behind any CPU-bound work on the machine. Measured on a 12-core Mac with a
//! load average over 100, a default-priority thread woke 4 to 6 ms late at the 99th
//! percentile and up to 19 ms late, against 0.02 ms for a user-interactive one; under
//! heavier load macOS disabled the tap for answering too slowly.

/// How urgent a thread's work is, in macOS quality-of-service terms.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum InputThreadPriority {
    /// The thread macOS waits on before it delivers a key.
    Interactive,
    /// Work a person is waiting for right after the key: running the chord.
    Initiated,
}

/// Raises the calling thread's priority. A failure leaves the thread as it was and
/// is logged: input still works, only slower when the machine is busy.
pub(super) fn raise_current_thread(priority: InputThreadPriority) {
    let class = match priority {
        InputThreadPriority::Interactive => libc::qos_class_t::QOS_CLASS_USER_INTERACTIVE,
        InputThreadPriority::Initiated => libc::qos_class_t::QOS_CLASS_USER_INITIATED,
    };
    // SAFETY: changes only the calling thread's scheduling class.
    let status = unsafe { libc::pthread_set_qos_class_self_np(class, 0) };
    if status != 0 {
        log::warn!(
            "Could not raise the priority of thread {:?} to {priority:?} (error {status})",
            std::thread::current().name()
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn current_class() -> libc::qos_class_t {
        let mut class = libc::qos_class_t::QOS_CLASS_UNSPECIFIED;
        let mut relative = 0;
        // SAFETY: both outputs point to live storage.
        let status = unsafe {
            libc::pthread_get_qos_class_np(libc::pthread_self(), &mut class, &mut relative)
        };
        assert_eq!(status, 0);
        class
    }

    #[test]
    fn raises_the_calling_thread_only() {
        let raised = std::thread::spawn(|| {
            raise_current_thread(InputThreadPriority::Interactive);
            current_class() as u32
        })
        .join()
        .unwrap();
        assert_eq!(
            raised,
            libc::qos_class_t::QOS_CLASS_USER_INTERACTIVE as u32
        );
        let initiated = std::thread::spawn(|| {
            raise_current_thread(InputThreadPriority::Initiated);
            current_class() as u32
        })
        .join()
        .unwrap();
        assert_eq!(
            initiated,
            libc::qos_class_t::QOS_CLASS_USER_INITIATED as u32
        );
        let untouched = std::thread::spawn(|| current_class() as u32)
            .join()
            .unwrap();
        assert_ne!(
            untouched,
            libc::qos_class_t::QOS_CLASS_USER_INTERACTIVE as u32
        );
    }
}
