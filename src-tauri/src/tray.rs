//! 系统托盘。关闭主窗口后应用驻留托盘，dsh 子进程保持运行。
//!
//! 菜单：打开主窗口 / 重启 dsh 服务 / 退出。左键单击显示主窗口。

use tauri::{
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Manager,
};

const TRAY_ID: &str = "main-tray";

pub fn create(app: &AppHandle) -> tauri::Result<()> {
    let show = MenuItem::with_id(app, "show", "打开主窗口", true, None::<&str>)?;
    let restart = MenuItem::with_id(app, "restart_dsh", "重启 dsh 服务", true, None::<&str>)?;
    let sep = PredefinedMenuItem::separator(app)?;
    let quit = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;

    let menu = Menu::with_items(app, &[&show, &restart, &sep, &quit])?;

    let mut builder = TrayIconBuilder::with_id(TRAY_ID)
        .tooltip("DSH Agent")
        .menu(&menu)
        .show_menu_on_left_click(false);

    // 复用窗口图标
    if let Some(icon) = app.default_window_icon().cloned() {
        builder = builder.icon(icon);
    }

    builder
        .on_menu_event(|app, event| {
            let app = app.clone();
            let id = event.id.as_ref().to_string();
            tauri::async_runtime::spawn(async move {
                match id.as_str() {
                    "show" => show_main(&app),
                    "restart_dsh" => {
                        let _ = crate::process::restart_dsh_from_tray(&app).await;
                    }
                    "quit" => crate::quit(&app),
                    _ => {}
                }
            });
        })
        .on_tray_icon_event(|tray, event| {
            // 左键单击显示主窗口，符合 Windows 习惯
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                show_main(tray.app_handle());
            }
        })
        .build(app)?;

    Ok(())
}

/// 显示主窗口（隐藏后重新唤出）。供托盘回调与 single-instance 调用。
pub fn show_main(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}
