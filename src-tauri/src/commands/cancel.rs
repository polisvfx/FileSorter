use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// Cooperative cancellation for the running sort.
///
/// Held as an `Arc` so the flag can be cloned into the blocking thread the sort
/// runs on, leaving the main thread free to service `cancel_sort`.
pub struct CancelState {
    flag: Arc<AtomicBool>,
}

impl CancelState {
    pub fn new() -> Self {
        Self {
            flag: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Clear the flag at the start of a run.
    pub fn reset(&self) {
        self.flag.store(false, Ordering::Relaxed);
    }

    pub fn flag(&self) -> Arc<AtomicBool> {
        Arc::clone(&self.flag)
    }
}

impl Default for CancelState {
    fn default() -> Self {
        Self::new()
    }
}

#[tauri::command]
pub fn cancel_sort(state: tauri::State<'_, CancelState>) {
    state.flag.store(true, Ordering::Relaxed);
}
