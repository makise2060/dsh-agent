//! 关闭行为命令（托盘常驻相关）。

use crate::settings;
use tauri::AppHandle;

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
