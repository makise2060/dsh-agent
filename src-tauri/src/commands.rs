//! 关闭行为与通知开关命令。

use crate::settings;
use tauri::{AppHandle, Manager};

/// 关闭确认框的三选：quit / tray / cancel。
/// action 为 cancel 时不记住（否则用户下次永远关不掉窗口）。
#[tauri::command]
pub fn resolve_close(app: AppHandle, action: String, remember: bool) {
    if remember && action != "cancel" {
        settings::set_close_action(&app, &action);
    }
    match action.as_str() {
        "quit" => crate::quit(&app),
        "tray" => crate::hide_main(&app),
        _ => {}
    }
}

#[tauri::command]
pub fn get_close_action(app: AppHandle) -> Option<String> {
    settings::get_close_action(&app)
}

#[tauri::command]
pub fn set_close_action(app: AppHandle, action: String) {
    settings::set_close_action(&app, &action);
}

/// 任务完成通知开关（读持久化设置）。
#[tauri::command]
pub fn get_notify_on_done(app: AppHandle) -> bool {
    settings::get_notify_on_done(&app)
}

/// 任务完成通知开关（写持久化设置 + 实时更新 watcher 原子开关）。
#[tauri::command]
pub fn set_notify_on_done(app: AppHandle, enabled: bool) {
    settings::set_notify_on_done(&app, enabled);
    app.state::<crate::watcher::NotifyEnabled>().set(enabled);
}
