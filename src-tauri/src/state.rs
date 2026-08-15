use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU32, Ordering};
use tokio::process::Child;
use tokio::sync::Mutex;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ProcessState {
    pub status: String,
    pub url: Option<String>,
    pub port: Option<u16>,
    pub pid: Option<u32>,
    pub error: Option<String>,
    pub started_at: Option<String>,
}

impl Default for ProcessState {
    fn default() -> Self {
        Self {
            status: "NotStarted".to_string(),
            url: None,
            port: None,
            pid: None,
            error: None,
            started_at: None,
        }
    }
}

#[derive(Default)]
pub struct AppState {
    pub child: Mutex<Option<Child>>,
    pub process_state: Mutex<ProcessState>,
    /// Lock-free mirror of the dsh root pid, updated right after spawn and
    /// cleared after every kill. Lets synchronous contexts (window close)
    /// reliably kill the process tree even when the async state mutex is
    /// contended, and covers the "Starting" window where ProcessState.pid
    /// is still None.
    pub dsh_pid: AtomicU32,
}

impl AppState {
    pub fn set_dsh_pid(&self, pid: u32) {
        self.dsh_pid.store(pid, Ordering::Relaxed);
    }

    pub fn get_dsh_pid(&self) -> u32 {
        self.dsh_pid.load(Ordering::Relaxed)
    }
}
