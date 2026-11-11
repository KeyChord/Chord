//! How soon Space follows Caps when someone starts Chord.
//!
//! The hotkey fallback (`caps_hotkey`) can only listen for Space once it knows Caps is
//! down, which takes it about half a millisecond. Whether real presses ever come that
//! close depends on the person and on the keyboard, which reports its keys in slices
//! (8 ms on a MacBook's built-in keyboard): two keys that go down within one slice
//! arrive together. This keeps the numbers that answer it, and puts them in the log
//! sparingly, since the log is small.

use parking_lot::Mutex;
use std::time::{Duration, Instant};

/// A common keyboard report interval. Presses closer than this can share a report.
const REPORT_INTERVAL: Duration = Duration::from_millis(8);
/// Below this, the hotkey fallback may not have Space registered in time.
const REGISTRATION_REACH: Duration = Duration::from_millis(1);
/// A summary goes to the log every this many activations.
const SUMMARY_EVERY: u64 = 50;

/// When a key event happened.
#[derive(Clone, Copy, Debug)]
pub struct KeyTime {
    /// The time the keyboard driver stamped on the event, in nanoseconds, where the
    /// listener has it. It is the one that matters: what the system saw.
    pub event_ns: Option<u64>,
    /// When the event reached Chord, which a busy system delays.
    pub arrived: Instant,
}

/// One activation's gap, and whether it is worth a line in the log.
#[derive(Debug, PartialEq)]
pub struct Gap {
    /// By the events' own timestamps when both have one, else by arrival.
    pub gap: Duration,
    pub arrival_gap: Duration,
    pub shortest_so_far: bool,
    pub summary_due: bool,
}

#[derive(Default)]
struct State {
    caps_at: Option<KeyTime>,
    activations: u64,
    within_report_interval: u64,
    within_registration_reach: u64,
    shortest: Option<Duration>,
}

#[derive(Default)]
pub struct CapsSpaceGaps(Mutex<State>);

impl CapsSpaceGaps {
    /// Caps went down, with Caps Lock off: Space may follow.
    pub fn caps_pressed(&self, at: KeyTime) {
        self.0.lock().caps_at = Some(at);
    }

    /// Caps went up, or the press cannot start Chord.
    pub fn caps_cleared(&self) {
        self.0.lock().caps_at = None;
    }

    /// Space started Chord. Returns the gap from the Caps press, once per press:
    /// Space repeating while both are held is not another activation.
    pub fn space_followed(&self, at: KeyTime) -> Option<Gap> {
        let mut state = self.0.lock();
        let caps = state.caps_at.take()?;
        let arrival_gap = at.arrived.saturating_duration_since(caps.arrived);
        let gap = match (caps.event_ns, at.event_ns) {
            (Some(caps), Some(space)) => Duration::from_nanos(space.saturating_sub(caps)),
            _ => arrival_gap,
        };
        state.activations += 1;
        if gap < REPORT_INTERVAL {
            state.within_report_interval += 1;
        }
        if gap < REGISTRATION_REACH {
            state.within_registration_reach += 1;
        }
        let shortest_so_far = state.shortest.is_none_or(|shortest| gap < shortest);
        if shortest_so_far {
            state.shortest = Some(gap);
        }
        Some(Gap {
            gap,
            arrival_gap,
            shortest_so_far,
            summary_due: state.activations % SUMMARY_EVERY == 0,
        })
    }

    /// The numbers so far, or `None` before the first activation.
    pub fn summary(&self) -> Option<String> {
        let state = self.0.lock();
        let shortest = state.shortest?;
        Some(format!(
            "{} activation(s), shortest {}, {} under {} ms, {} under {} ms",
            state.activations,
            milliseconds(shortest),
            state.within_report_interval,
            REPORT_INTERVAL.as_millis(),
            state.within_registration_reach,
            REGISTRATION_REACH.as_millis(),
        ))
    }
}

pub fn milliseconds(duration: Duration) -> String {
    format!("{:.2} ms", duration.as_secs_f64() * 1_000.0)
}

/// An event timestamp in nanoseconds. Core Graphics documents nanoseconds, but at the
/// HID tap on Apple silicon the value is in Mach ticks (about 41.7 ns each), measured
/// on macOS 27. Rather than assume either, this goes by which reading of the current
/// time the stamp is closer to: an event is always a moment old, never seconds.
pub fn event_time_ns(stamp: u64, now_ticks: u64, numer: u32, denom: u32) -> u64 {
    let to_ns = |ticks: u64| {
        u64::try_from(u128::from(ticks) * u128::from(numer) / u128::from(denom.max(1)))
            .unwrap_or(u64::MAX)
    };
    if stamp.abs_diff(now_ticks) <= stamp.abs_diff(to_ns(now_ticks)) {
        to_ns(stamp)
    } else {
        stamp
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(start: Instant, event_ms: Option<f64>, arrived_ms: f64) -> KeyTime {
        KeyTime {
            event_ns: event_ms.map(|ms| (ms * 1_000_000.0) as u64),
            arrived: start + Duration::from_secs_f64(arrived_ms / 1_000.0),
        }
    }

    #[test]
    fn the_gap_is_between_the_events_not_their_arrival() {
        let gaps = CapsSpaceGaps::default();
        let start = Instant::now();
        // A busy moment delivered both late and close together.
        gaps.caps_pressed(at(start, Some(1_000.0), 70.0));
        let gap = gaps.space_followed(at(start, Some(1_040.0), 70.2)).unwrap();
        assert_eq!(gap.gap, Duration::from_millis(40));
        assert_eq!(gap.arrival_gap, Duration::from_micros(200));
        assert!(gap.shortest_so_far);
    }

    #[test]
    fn without_event_times_the_arrival_gap_stands_in() {
        let gaps = CapsSpaceGaps::default();
        let start = Instant::now();
        gaps.caps_pressed(at(start, None, 0.0));
        let gap = gaps.space_followed(at(start, None, 25.0)).unwrap();
        assert_eq!(gap.gap, Duration::from_millis(25));
        // One side alone is not enough: the two clocks share no origin.
        gaps.caps_pressed(at(start, Some(5.0), 100.0));
        let gap = gaps.space_followed(at(start, None, 130.0)).unwrap();
        assert_eq!(gap.gap, Duration::from_millis(30));
    }

    #[test]
    fn a_press_counts_once_and_only_after_caps() {
        let gaps = CapsSpaceGaps::default();
        let start = Instant::now();
        assert_eq!(gaps.space_followed(at(start, None, 0.0)), None);
        gaps.caps_pressed(at(start, None, 0.0));
        assert!(gaps.space_followed(at(start, None, 30.0)).is_some());
        // Space repeating while Caps is still held.
        assert_eq!(gaps.space_followed(at(start, None, 500.0)), None);
        gaps.caps_pressed(at(start, None, 600.0));
        gaps.caps_cleared();
        assert_eq!(gaps.space_followed(at(start, None, 620.0)), None);
    }

    #[test]
    fn the_summary_counts_the_close_calls() {
        let gaps = CapsSpaceGaps::default();
        assert_eq!(gaps.summary(), None);
        let start = Instant::now();
        let mut shortest = Vec::new();
        for (index, gap_ms) in [40.0, 55.0, 6.5, 0.3, 12.0].into_iter().enumerate() {
            let pressed = index as f64 * 1_000.0;
            gaps.caps_pressed(at(start, Some(pressed), pressed));
            let gap = gaps
                .space_followed(at(start, Some(pressed + gap_ms), pressed + gap_ms))
                .unwrap();
            shortest.push(gap.shortest_so_far);
            assert!(!gap.summary_due);
        }
        assert_eq!(shortest, [true, false, true, true, false]);
        assert_eq!(
            gaps.summary().unwrap(),
            "5 activation(s), shortest 0.30 ms, 2 under 8 ms, 1 under 1 ms"
        );
    }

    #[test]
    fn a_summary_is_due_every_fiftieth_activation() {
        let gaps = CapsSpaceGaps::default();
        let start = Instant::now();
        let due: Vec<u64> = (1..=120)
            .filter(|index| {
                let pressed = *index as f64 * 1_000.0;
                gaps.caps_pressed(at(start, None, pressed));
                gaps.space_followed(at(start, None, pressed + 30.0))
                    .unwrap()
                    .summary_due
            })
            .collect();
        assert_eq!(due, [50, 100]);
    }

    #[test]
    fn event_stamps_are_read_in_whichever_unit_they_come() {
        // Apple silicon: 125/3 nanoseconds per tick. An hour after boot:
        let now_ns: u64 = 3_600_000_000_000;
        let now_ticks = now_ns * 3 / 125;
        // A stamp in ticks, 200 microseconds old.
        let stamp_ticks = (now_ns - 200_000) * 3 / 125;
        assert_eq!(
            event_time_ns(stamp_ticks, now_ticks, 125, 3) / 1_000,
            (now_ns - 200_000) / 1_000
        );
        // The same moment already in nanoseconds is left alone.
        assert_eq!(
            event_time_ns(now_ns - 200_000, now_ticks, 125, 3),
            now_ns - 200_000
        );
        // Where a tick is a nanosecond, there is nothing to tell apart.
        assert_eq!(
            event_time_ns(now_ns - 200_000, now_ns, 1, 1),
            now_ns - 200_000
        );
    }
}
