//! 用户偏好持久化：关闭行为（quit / tray）等。
//!
//! 存储为 JSON 文件，放在应用配置目录（%APPDATA%\com.dsh-agent.app\settings.json）。
//! 读写都尽量容错：配置损坏不应阻止应用启动。

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

const SETTINGS_FILE: &str = "settings.json";

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct Settings {
    /// 记住的关闭行为：Some("quit") / Some("tray")。None = 每次询问。
    close_action: Option<String>,
    /// 任务完成桌面通知开关。None = 默认开启。
    notify_on_done: Option<bool>,
}

fn settings_path(app: &AppHandle) -> Option<PathBuf> {
    app.path()
        .app_config_dir()
        .ok()
        .map(|dir| dir.join(SETTINGS_FILE))
}

fn load(app: &AppHandle) -> Settings {
    let Some(path) = settings_path(app) else {
        return Settings::default();
    };
    let Ok(raw) = std::fs::read_to_string(path) else {
        return Settings::default();
    };
    serde_json::from_str(&raw).unwrap_or_default()
}

fn save(app: &AppHandle, settings: &Settings) {
    let Some(path) = settings_path(app) else {
        return;
    };
    let Some(dir) = path.parent() else {
        return;
    };
    if let Err(e) = std::fs::create_dir_all(dir) {
        log::warn!("settings: 创建配置目录失败: {}", e);
        return;
    }
    let Ok(raw) = serde_json::to_string_pretty(settings) else {
        return;
    };
    if let Err(e) = std::fs::write(&path, raw) {
        log::warn!("settings: 写入 {} 失败: {}", path.display(), e);
    }
}

/// 记住的关闭行为：Some("quit") / Some("tray") / None（每次询问）
pub fn get_close_action(app: &AppHandle) -> Option<String> {
    load(app).close_action.filter(|a| !a.is_empty())
}

/// 记住关闭行为。action 为空或 "cancel" 时清除记忆（cancel 永不记住）。
pub fn set_close_action(app: &AppHandle, action: &str) {
    let mut settings = load(app);
    if action.is_empty() || action == "cancel" {
        settings.close_action = None;
    } else {
        settings.close_action = Some(action.to_string());
    }
    save(app, &settings);
}

/// 任务完成通知开关（默认开启）。
pub fn get_notify_on_done(app: &AppHandle) -> bool {
    load(app).notify_on_done.unwrap_or(true)
}

pub fn set_notify_on_done(app: &AppHandle, enabled: bool) {
    let mut settings = load(app);
    settings.notify_on_done = Some(enabled);
    save(app, &settings);
}
