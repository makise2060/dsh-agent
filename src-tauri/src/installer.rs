use tauri::{AppHandle, Emitter};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;

#[derive(serde::Serialize, Clone)]
pub struct InstallProgress {
    pub stage: String,
    pub message: String,
    pub percent: Option<u32>,
}

#[tauri::command]
pub async fn install_dsh(app: AppHandle) -> Result<(), String> {
    // Emit starting
    let _ = app.emit(
        "install-progress",
        InstallProgress {
            stage: "starting".to_string(),
            message: "正在安装 @deepseek-ai/dsh...".to_string(),
            percent: None,
        },
    );

    let mut child = Command::new("npm")
        .args(["install", "-g", "@deepseek-ai/dsh@latest"])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to run npm install: {}", e))?;

    // Stream stdout
    if let Some(stdout) = child.stdout.take() {
        let reader = BufReader::new(stdout);
        let mut lines = reader.lines();
        while let Ok(Some(line)) = lines.next_line().await {
            let _ = app.emit(
                "install-progress",
                InstallProgress {
                    stage: "progress".to_string(),
                    message: line,
                    percent: None,
                },
            );
        }
    }

    // Stream stderr
    if let Some(stderr) = child.stderr.take() {
        let reader = BufReader::new(stderr);
        let mut lines = reader.lines();
        while let Ok(Some(line)) = lines.next_line().await {
            let _ = app.emit(
                "install-progress",
                InstallProgress {
                    stage: "progress".to_string(),
                    message: line,
                    percent: None,
                },
            );
        }
    }

    let status = child
        .wait()
        .await
        .map_err(|e| format!("Failed to wait for npm install: {}", e))?;

    if status.success() {
        let _ = app.emit(
            "install-progress",
            InstallProgress {
                stage: "done".to_string(),
                message: "dsh 安装成功!".to_string(),
                percent: Some(100),
            },
        );
        Ok(())
    } else {
        let code = status.code().unwrap_or(-1);
        let _ = app.emit(
            "install-progress",
            InstallProgress {
                stage: "error".to_string(),
                message: format!("安装失败 (exit code: {})", code),
                percent: None,
            },
        );
        Err(format!("npm install failed with exit code {}", code))
    }
}
