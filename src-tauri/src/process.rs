use crate::cmd_ext::hidden_command;
use crate::state::{AppState, ProcessState};
use regex::Regex;
use std::time::Duration;
use tauri::{AppHandle, Emitter, State};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::time::timeout;

const DSH_START_TIMEOUT: Duration = Duration::from_secs(30);
static URL_REGEX: &str = r"^dsh web:\s+http://127\.0\.0\.1:(\d+)";

async fn update_state(state: &State<'_, AppState>, new_state: ProcessState, app: &AppHandle) {
    {
        let mut s = state.process_state.lock().await;
        *s = new_state.clone();
    }
    let _ = app.emit("process-state-changed", &new_state);
}

async fn get_state(state: &State<'_, AppState>) -> ProcessState {
    state.process_state.lock().await.clone()
}

#[cfg(target_os = "windows")]
async fn kill_process_tree(pid: u32) -> Result<(), String> {
    hidden_command("taskkill")
        .args(["/pid", &pid.to_string(), "/T", "/F"])
        .output()
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg(not(target_os = "windows"))]
async fn kill_process_tree(pid: u32) -> Result<(), String> {
    // Try SIGTERM first
    hidden_command("kill")
        .args(["-TERM", &pid.to_string()])
        .output()
        .await
        .map_err(|e| e.to_string())?;
    tokio::time::sleep(Duration::from_secs(3)).await;
    // Force kill if still alive
    hidden_command("kill")
        .args(["-KILL", &pid.to_string()])
        .output()
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn start_dsh(
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<ProcessState, String> {
    // If already running, return current state
    {
        let current = get_state(&state).await;
        if current.status == "Running" {
            return Ok(current);
        }
    }

    // Set Starting state
    let starting = ProcessState {
        status: "Starting".to_string(),
        ..Default::default()
    };
    update_state(&state, starting, &app).await;

    // Spawn dsh web --port 0
    // On Windows, `dsh` is typically a .cmd shim; use cmd.exe /C to resolve it
    #[cfg(target_os = "windows")]
    let mut child = hidden_command("cmd.exe")
        .args(["/C", "dsh", "web", "--port", "0"])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to spawn dsh: {}", e))?;

    #[cfg(not(target_os = "windows"))]
    let mut child = hidden_command("dsh")
        .args(["web", "--port", "0"])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to spawn dsh: {}", e))?;

    let pid = child.id().unwrap_or(0);

    // Take stdout before storing child
    let stdout = child.stdout.take().ok_or("No stdout")?;

    // Store child handle (without stdout since we took it)
    {
        let mut c = state.child.lock().await;
        *c = Some(child);
    }

    // Parse stdout for URL line
    let re = Regex::new(URL_REGEX).map_err(|e| e.to_string())?;
    let reader = BufReader::new(stdout);
    let mut lines = reader.lines();

    let result = timeout(DSH_START_TIMEOUT, async {
        loop {
            match lines.next_line().await {
                Ok(Some(line)) => {
                    log::info!("dsh stdout: {}", line);
                    if let Some(caps) = re.captures(&line) {
                        let port: u16 = caps[1]
                            .parse()
                            .map_err(|e: std::num::ParseIntError| e.to_string())?;
                        return Ok(port);
                    }
                }
                Ok(None) => {
                    return Err::<u16, String>(
                        "dsh process exited before URL was printed".to_string(),
                    );
                }
                Err(e) => {
                    return Err(format!("Failed to read stdout: {}", e));
                }
            }
        }
    })
    .await;

    match result {
        Ok(Ok(port)) => {
            let running = ProcessState {
                status: "Running".to_string(),
                url: Some(format!("http://127.0.0.1:{}", port)),
                port: Some(port),
                pid: Some(pid),
                error: None,
                started_at: Some(chrono_now()),
            };
            update_state(&state, running.clone(), &app).await;
            Ok(running)
        }
        Ok(Err(e)) => {
            let failed = ProcessState {
                status: "Failed".to_string(),
                error: Some(e.clone()),
                pid: Some(pid),
                ..Default::default()
            };
            update_state(&state, failed.clone(), &app).await;
            if pid > 0 {
                let _ = kill_process_tree(pid).await;
            }
            Err(e)
        }
        Err(_) => {
            let error_msg = "dsh web 启动超时 (30s)，请检查环境配置".to_string();
            let failed = ProcessState {
                status: "Failed".to_string(),
                error: Some(error_msg.clone()),
                pid: Some(pid),
                ..Default::default()
            };
            update_state(&state, failed.clone(), &app).await;
            if pid > 0 {
                let _ = kill_process_tree(pid).await;
            }
            Err(error_msg)
        }
    }
}

#[tauri::command]
pub async fn stop_dsh(state: State<'_, AppState>, app: AppHandle) -> Result<(), String> {
    let pid = {
        let current = get_state(&state).await;
        if current.status == "Stopped" || current.status == "NotStarted" {
            return Ok(());
        }
        current.pid.unwrap_or(0)
    };

    update_state(
        &state,
        ProcessState {
            status: "Stopping".to_string(),
            ..Default::default()
        },
        &app,
    )
    .await;

    if pid > 0 {
        kill_process_tree(pid).await?;
    }

    // Clear child handle
    {
        let mut c = state.child.lock().await;
        *c = None;
    }

    update_state(
        &state,
        ProcessState {
            status: "Stopped".to_string(),
            ..Default::default()
        },
        &app,
    )
    .await;

    Ok(())
}

#[tauri::command]
pub async fn get_dsh_status(state: State<'_, AppState>) -> Result<ProcessState, String> {
    Ok(get_state(&state).await)
}

#[tauri::command]
pub async fn restart_dsh(
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<ProcessState, String> {
    let _ = stop_dsh(state.clone(), app.clone()).await;
    // Small delay to ensure port is released
    tokio::time::sleep(Duration::from_secs(1)).await;
    start_dsh(state, app).await
}

/// Called on app exit to ensure dsh is killed
pub async fn cleanup_on_exit(state: &AppState) {
    let pid = state.process_state.lock().await.pid.unwrap_or(0);
    if pid > 0 {
        let _ = kill_process_tree(pid).await;
    }
}

fn chrono_now() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format!("{}", secs)
}
