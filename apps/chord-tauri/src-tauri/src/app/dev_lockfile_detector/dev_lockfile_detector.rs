use super::gate::InputGate;
use nject::injectable;
use tauri::AppHandle;

#[injectable]
pub struct DevLockfileDetector {
    // Ownership changes wait for callbacks and queued handlers to finish before
    // releasing the OS lease. Queued events retain their originating generation.
    #[inject(InputGate::default())]
    pub(super) gate: InputGate,
    pub(super) handle: AppHandle,
}

impl DevLockfileDetector {
    pub fn with_input_owner<T>(&self, f: impl FnOnce(u64) -> T) -> Option<T> {
        self.gate.capture(f)
    }
    pub fn with_input_generation<T>(&self, expected: u64, f: impl FnOnce() -> T) -> Option<T> {
        self.gate.process(expected, f)
    }
}
