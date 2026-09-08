use std::sync::atomic::{AtomicBool, Ordering};

/// Reserving before spawning prevents concurrent permission refreshes from
/// installing duplicate listeners. Startup failure/thread exit permits retry.
#[derive(Default)]
pub(super) struct Registration(AtomicBool);

impl Registration {
    pub(super) const fn new() -> Self {
        Self(AtomicBool::new(false))
    }
    pub(super) fn acquire(&self) -> Option<RegistrationGuard<'_>> {
        self.0
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .ok()
            .map(|_| RegistrationGuard(self))
    }
    pub(super) fn is_registered(&self) -> bool {
        self.0.load(Ordering::Acquire)
    }
}

pub(super) struct RegistrationGuard<'a>(&'a Registration);
impl Drop for RegistrationGuard<'_> {
    fn drop(&mut self) {
        self.0.0.store(false, Ordering::Release);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn concurrent_registration_has_one_winner_and_can_retry_after_exit() {
        let registration = Registration::new();
        let guard = registration.acquire().unwrap();
        std::thread::scope(|scope| {
            for _ in 0..16 {
                let registration = &registration;
                scope.spawn(move || assert!(registration.acquire().is_none()));
            }
        });
        drop(guard);
        assert!(registration.acquire().is_some());
    }
}
