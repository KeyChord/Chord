use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::{Duration, Instant};

/// Live status of the keyboard event tap, written by the tap thread and read
/// by the supervisor and the input-ownership lease.
#[derive(Default)]
pub struct InputHealth {
    tap_running: AtomicBool,
    tap_enabled: AtomicBool,
    /// Set while the tap is being replaced, so the swap is not read as an outage.
    tap_restarting: AtomicBool,
    /// How many events of the remapped Caps key the tap received and let through.
    caps_events_forwarded: AtomicU64,
    /// The same for Space.
    space_events_forwarded: AtomicU64,
    /// When a stalled key was last written to the log, in nanoseconds of the tap
    /// thread's clock; zero before the first.
    stall_reported_at: AtomicU64,
    /// Stalled keys since that log line.
    stalls_unreported: AtomicU64,
}

/// A key that waited at least this long for the tap is a stall worth a log line: it
/// is long enough to feel, and a fraction of what makes macOS disable the tap.
pub(super) const STALL_THRESHOLD: Duration = Duration::from_millis(100);
/// Stalls come in bursts on a struggling machine; the log keeps one line per burst.
const STALL_REPORT_INTERVAL: Duration = Duration::from_secs(30);

impl InputHealth {
    /// The tap exists, or is being replaced right now.
    pub fn tap_running(&self) -> bool {
        self.tap_running.load(Ordering::Acquire) || self.tap_restarting.load(Ordering::Acquire)
    }

    pub(super) fn set_tap_restarting(&self, restarting: bool) {
        self.tap_restarting.store(restarting, Ordering::Release);
    }

    /// The tap received an event of the remapped Caps key and did not swallow it.
    pub(super) fn note_caps_event_forwarded(&self) {
        self.caps_events_forwarded.fetch_add(1, Ordering::AcqRel);
    }

    pub(super) fn caps_events_forwarded(&self) -> u64 {
        self.caps_events_forwarded.load(Ordering::Acquire)
    }

    /// The tap received a Space event and did not swallow it.
    pub(super) fn note_space_event_forwarded(&self) {
        self.space_events_forwarded.fetch_add(1, Ordering::AcqRel);
    }

    pub(super) fn space_events_forwarded(&self) -> u64 {
        self.space_events_forwarded.load(Ordering::Acquire)
    }

    /// A key waited too long at the tap. Returns how many stalls went unreported
    /// before this one when it is time for a log line, and `None` otherwise. Called
    /// on the tap thread; only atomics.
    pub(super) fn note_stall(&self, now_ns: u64) -> Option<u64> {
        let last = self.stall_reported_at.load(Ordering::Acquire);
        let interval = STALL_REPORT_INTERVAL.as_nanos() as u64;
        if last != 0 && now_ns.saturating_sub(last) < interval {
            self.stalls_unreported.fetch_add(1, Ordering::AcqRel);
            return None;
        }
        self.stall_reported_at.store(now_ns.max(1), Ordering::Release);
        Some(self.stalls_unreported.swap(0, Ordering::AcqRel))
    }

    pub fn tap_enabled(&self) -> bool {
        self.tap_enabled.load(Ordering::Acquire)
    }

    pub(super) fn set_tap_running(&self, running: bool) {
        self.tap_running.store(running, Ordering::Release);
        if !running {
            self.tap_enabled.store(false, Ordering::Release);
        }
    }

    pub(super) fn set_tap_enabled(&self, enabled: bool) {
        self.tap_enabled.store(enabled, Ordering::Release);
    }
}

/// Exponential retry schedule for restarting a listener after it failed.
#[derive(Debug, Default)]
pub struct Backoff {
    failures: u32,
    retry_at: Option<Instant>,
}

impl Backoff {
    const DELAYS: [Duration; 6] = [
        Duration::from_secs(1),
        Duration::from_secs(2),
        Duration::from_secs(4),
        Duration::from_secs(8),
        Duration::from_secs(16),
        Duration::from_secs(30),
    ];

    pub fn ready(&self, now: Instant) -> bool {
        self.retry_at.is_none_or(|retry_at| now >= retry_at)
    }

    pub fn failed(&mut self, now: Instant) -> Duration {
        let delay = Self::DELAYS[(self.failures as usize).min(Self::DELAYS.len() - 1)];
        self.failures = self.failures.saturating_add(1);
        self.retry_at = Some(now + delay);
        delay
    }

    pub fn succeeded(&mut self) {
        self.failures = 0;
        self.retry_at = None;
    }

    pub fn failures(&self) -> u32 {
        self.failures
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stalls_are_reported_once_per_burst() {
        let health = InputHealth::default();
        let second = 1_000_000_000;
        assert_eq!(health.note_stall(5 * second), Some(0));
        assert_eq!(health.note_stall(6 * second), None);
        assert_eq!(health.note_stall(20 * second), None);
        // The next burst says how many the last one hid.
        assert_eq!(health.note_stall(36 * second), Some(2));
        assert_eq!(health.note_stall(37 * second), None);
    }

    #[test]
    fn backoff_grows_then_resets() {
        let mut backoff = Backoff::default();
        let now = Instant::now();
        assert!(backoff.ready(now));
        assert_eq!(backoff.failed(now), Duration::from_secs(1));
        assert!(!backoff.ready(now));
        assert!(backoff.ready(now + Duration::from_secs(1)));
        assert_eq!(backoff.failed(now), Duration::from_secs(2));
        for _ in 0..10 {
            backoff.failed(now);
        }
        assert_eq!(backoff.failed(now), Duration::from_secs(30));
        backoff.succeeded();
        assert!(backoff.ready(now));
        assert_eq!(backoff.failed(now), Duration::from_secs(1));
    }
}
