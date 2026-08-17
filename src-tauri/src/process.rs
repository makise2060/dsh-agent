use crate::cmd_ext::hidden_command;
use crate::state::{AppState, ProcessState};
use regex::Regex;
use std::time::Duration;
use tauri::{AppHandle, Emitter, State};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::time::timeout;

const DSH_START_TIMEOUT: Duration = Duration::from_secs(120);
const MAX_START_ATTEMPTS: u32 = 3;
const RETRY_DELAY: Duration = Duration::from_secs(3);
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
    update_state(&state, starting.clone(), &app).await;

    // dsh's profile heal is idempotent and often self-heals on the next run
    // (e.g. a broken fallback symlink gets rebuilt), so retry a few times
    // before giving up instead of forcing the user to click "retry" manually.
    let mut last_err = String::new();
    for attempt in 1..=MAX_START_ATTEMPTS {
        match start_dsh_once(&state, &app).await {
            Ok(running) => return Ok(running),
            Err(e) => {
                log::warn!(
                    "dsh start attempt {}/{} failed: {}",
                    attempt,
                    MAX_START_ATTEMPTS,
                    e
                );
                last_err = e;
                if attempt < MAX_START_ATTEMPTS {
                    // dsh's fallback dir (profiles/node_modules) is fully
                    // managed by dsh — every entry is a symlink it rebuilds on
                    // boot. Wiping it before the retry forces a clean heal and
                    // fixes dangling-link failures that dsh's own heal skips
                    // (it only compares link targets, never validates them).
                    let fallback =
                        crate::env::get_dsh_home().join("profiles").join("node_modules");
                    if fallback.exists() {
                        log::warn!(
                            "wiping dsh fallback {} before retry",
                            fallback.display()
                        );
                        let _ = std::fs::remove_dir_all(&fallback);
                    }
                    update_state(&state, starting.clone(), &app).await;
                    tokio::time::sleep(RETRY_DELAY).await;
                }
            }
        }
    }

    let failed = ProcessState {
        status: "Failed".to_string(),
        error: Some(last_err.clone()),
        ..Default::default()
    };
    update_state(&state, failed, &app).await;
    Err(last_err)
}

/// One single spawn-and-wait-for-URL attempt.
async fn start_dsh_once(
    state: &State<'_, AppState>,
    app: &AppHandle,
) -> Result<ProcessState, String> {
    // Spawn dsh web --port 0 via npx, so we don't depend on global install / PATH.
    // Resolve the real npx path first — version-manager shims (nvmd etc.) hang
    // in elevated/hidden contexts, which made the first launch after install fail.
    #[cfg(target_os = "windows")]
    let mut child = {
        let npx = crate::cmd_ext::resolve_global_bin("npx").await;
        hidden_command("cmd.exe")
            .args(["/C", npx.as_str(), "@deepseek-ai/dsh", "web", "--port", "0"])
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| {
                log::error!("Failed to spawn dsh: {}", e);
                format!("Failed to spawn dsh: {}", e)
            })?
    };

    #[cfg(not(target_os = "windows"))]
    let mut child = hidden_command("npx")
        .args(["@deepseek-ai/dsh", "web", "--port", "0"])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to spawn dsh: {}", e))?;

    let pid = child.id().unwrap_or(0);
    log::info!("dsh spawned (pid={})", pid);

    // Publish the pid immediately (lock-free), so window-close / app exit can
    // always find and kill the tree — even during "Starting" or when the
    // async state mutex is contended.
    state.set_dsh_pid(pid);

    // Take stdout before storing child
    let stdout = child.stdout.take().ok_or("No stdout")?;

    // Take stderr too — CRITICAL: if stderr is piped but never consumed,
    // the child blocks once the OS pipe buffer (4KB on Windows) fills up,
    // which stalls dsh before it can print its URL. Spawn a reader task
    // that drains stderr for the lifetime of the process.
    let stderr = child.stderr.take().ok_or("No stderr")?;
    let app_for_stderr = app.clone();

    // Keep the most recent stderr lines so a failed start can surface the
    // actual dsh error (e.g. ERR_MODULE_NOT_FOUND) instead of a bare
    // "process exited" message.
    let stderr_tail: std::sync::Arc<tokio::sync::Mutex<std::collections::VecDeque<String>>> =
        std::sync::Arc::new(tokio::sync::Mutex::new(std::collections::VecDeque::new()));
    let stderr_tail_for_task = stderr_tail.clone();

    // Windows Job Object (KILL_ON_JOB_CLOSE): the whole dsh tree dies with us
    // even if we are killed without a CloseRequested (crash / task manager).
    // The guard lives inside the stderr reader task — when the task ends the
    // job closes, and any surviving descendants are force-killed by the OS.
    #[cfg(target_os = "windows")]
    let job = crate::cmd_ext::JobGuard::create();
    #[cfg(target_os = "windows")]
    if let Some(job) = &job {
        job.assign(pid);
    }
    #[cfg(not(target_os = "windows"))]
    let job = ();

    tokio::spawn(async move {
        // Keep the job handle alive for the lifetime of this reader task.
        let _job_guard = job;
        let reader = BufReader::new(stderr);
        let mut lines = reader.lines();
        while let Ok(Some(line)) = lines.next_line().await {
            log::info!("dsh stderr: {}", line);
            let _ = app_for_stderr.emit("dsh-stdout", &format!("[stderr] {}", line));
            let mut tail = stderr_tail_for_task.lock().await;
            tail.push_back(line);
            while tail.len() > 30 {
                tail.pop_front();
            }
        }
    });

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
                    // Emit stdout line to frontend for real-time display
                    let _ = app.emit("dsh-stdout", &line);
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

    // Surface the last stderr lines so the user can see the real cause.
    let with_tail = |err: String| {
        let tail = stderr_tail.try_lock();
        if let Ok(tail) = tail {
            if !tail.is_empty() {
                let lines: Vec<String> = tail.iter().cloned().collect();
                return format!(
                    "{}\n--- dsh stderr (最近 {} 行) ---\n{}",
                    err,
                    lines.len(),
                    lines.join("\n")
                );
            }
        }
        err
    };

    match result {
        Ok(Ok(port)) => {
            // Wait for the HTTP server to be ready before returning
            let url = format!("http://127.0.0.1:{}", port);
            let ready = timeout(Duration::from_secs(15), async {
                let client = reqwest::Client::builder()
                    .timeout(Duration::from_secs(2))
                    .build()
                    .ok()?;
                for _ in 0..30 {
                    if client.get(&url).send().await.is_ok() {
                        return Some(());
                    }
                    tokio::time::sleep(Duration::from_millis(500)).await;
                }
                None
            })
            .await;

            // Even if health check times out, still return Running — the server might just be slow
            if ready.is_err() || ready.unwrap_or_default().is_none() {
                log::warn!(
                    "dsh HTTP health check on {} failed within 15s (server may still be slow)",
                    url
                );
            } else {
                log::info!("dsh HTTP server ready on {}", url);
            }

            let running = ProcessState {
                status: "Running".to_string(),
                url: Some(format!("http://127.0.0.1:{}", port)),
                port: Some(port),
                pid: Some(pid),
                error: None,
                started_at: Some(chrono_now()),
            };
            update_state(state, running.clone(), app).await;
            Ok(running)
        }
        Ok(Err(e)) => {
            log::error!("dsh failed to start: {}", e);
            let err = with_tail(e);
            let failed = ProcessState {
                status: "Failed".to_string(),
                error: Some(err.clone()),
                pid: Some(pid),
                ..Default::default()
            };
            update_state(state, failed, app).await;
            if pid > 0 {
                let _ = kill_process_tree(pid).await;
            }
            state.set_dsh_pid(0);
            Err(err)
        }
        Err(_) => {
            let error_msg = format!(
                "dsh web 启动超时 ({}s)，请检查上方日志。若为首次启动，初始化可能较慢",
                DSH_START_TIMEOUT.as_secs()
            );
            log::error!("{}", error_msg);
            let err = with_tail(error_msg);
            let failed = ProcessState {
                status: "Failed".to_string(),
                error: Some(err.clone()),
                pid: Some(pid),
                ..Default::default()
            };
            update_state(state, failed, app).await;
            if pid > 0 {
                let _ = kill_process_tree(pid).await;
            }
            state.set_dsh_pid(0);
            Err(err)
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
        // Fall back to the lock-free pid (covers "Starting", where
        // ProcessState.pid is still None).
        current.pid.unwrap_or(state.get_dsh_pid())
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
        log::info!("dsh stopped (pid={})", pid);
    }
    state.set_dsh_pid(0);

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

fn chrono_now() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format!("{}", secs)
}
