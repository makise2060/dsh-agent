use serde::{Deserialize, Serialize};
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
}
