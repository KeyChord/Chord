use std::sync::RwLock;

#[derive(Default)]
pub(super) struct InputGate(RwLock<Option<u64>>);

impl InputGate {
    /// Event taps must never wait for a handoff or slow controller operation.
    pub(super) fn capture<T>(&self, f: impl FnOnce(u64) -> T) -> Option<T> {
        let generation = self.0.try_read().ok()?;
        generation.map(f)
    }

    pub(super) fn process<T>(&self, expected: u64, f: impl FnOnce() -> T) -> Option<T> {
        let generation = self.0.read().expect("input ownership poisoned");
        (*generation == Some(expected)).then(f)
    }

    pub(super) fn transition(&self, next: Option<u64>, reset: impl FnOnce()) {
        // Drain controller work before the caller releases the OS owner lock.
        let mut generation = self.0.write().expect("input ownership poisoned");
        *generation = None;
        reset();
        *generation = next;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;
    use std::time::Duration;

    #[test]
    fn queued_events_cannot_cross_a_handoff_or_reactivation() {
        let gate = InputGate::default();
        assert_eq!(gate.capture(|generation| generation), None);
        gate.transition(Some(1), || {});
        let queued = gate.capture(|generation| generation).unwrap();
        assert_eq!(gate.process(queued, || 42), Some(42));
        gate.transition(None, || {});
        assert_eq!(gate.process(queued, || 42), None);
        gate.transition(Some(2), || {});
        assert_eq!(gate.process(queued, || 42), None);
        assert_eq!(gate.process(2, || 42), Some(42));
    }

    #[test]
    fn handoff_waits_for_in_flight_work_and_callbacks_do_not_block_during_reset() {
        let gate = InputGate::default();
        gate.transition(Some(1), || {});
        std::thread::scope(|scope| {
            let (started_tx, started_rx) = mpsc::channel();
            let (finish_tx, finish_rx) = mpsc::channel();
            let gate_ref = &gate;
            scope.spawn(move || {
                gate_ref.process(1, || {
                    started_tx.send(()).unwrap();
                    finish_rx.recv().unwrap();
                })
            });
            started_rx.recv().unwrap();
            let (reset_tx, reset_rx) = mpsc::channel();
            let (transition_tx, transition_rx) = mpsc::channel();
            scope.spawn(move || {
                transition_tx.send(()).unwrap();
                gate_ref.transition(None, || {
                    // try_read must return immediately while the writer owns the gate.
                    assert!(gate_ref.capture(|_| ()).is_none());
                    reset_tx.send(()).unwrap();
                });
            });
            transition_rx.recv().unwrap();
            assert!(reset_rx.recv_timeout(Duration::from_millis(30)).is_err());
            finish_tx.send(()).unwrap();
            reset_rx.recv_timeout(Duration::from_secs(2)).unwrap();
        });
        assert!(gate.capture(|_| ()).is_none());
    }
}
