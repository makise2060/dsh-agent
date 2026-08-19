//! 系统托盘。关闭主窗口后应用驻留托盘，dsh 子进程保持运行。
//!
//! 菜单：打开主窗口 / 重启 dsh 服务 / 退出。左键单击显示主窗口。

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::Duration;

use tauri::{
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Manager,
};

const TRAY_ID: &str = "main-tray";

/// 托盘图标是否正在闪烁。
static BLINKING: AtomicBool = AtomicBool::new(false);

/// 闪烁任务的代次。只有 BLINKING 一个标志会漏掉这条时序：
/// 任务 A 卡在 sleep → 用户看了窗口，标志置 false → A 还没醒，新任务完成
/// 又把标志置回 true 并启动任务 B → A 醒来读到 true 继续跑，两个任务同时
/// 切图标互相覆盖。每个任务记住自己启动时的代次，发现代次变了就退出。
static BLINK_GEN: AtomicU64 = AtomicU64::new(0);

const BLINK_INTERVAL: Duration = Duration::from_millis(600);

/// 开始闪烁托盘图标（在「有图标」和「无图标」之间交替，
/// `set_icon(None)` 会清空图标，正好得到一闪一闪的效果）。
/// 重复调用是安全的：已经在闪就直接返回。
pub fn start_blink(app: &AppHandle) {
    if BLINKING.swap(true, Ordering::SeqCst) {
        return;
    }
    let generation = BLINK_GEN.fetch_add(1, Ordering::SeqCst) + 1;

    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let icon = app.default_window_icon().cloned();
        let mut visible = false;

        while BLINKING.load(Ordering::SeqCst) && BLINK_GEN.load(Ordering::SeqCst) == generation {
            if let Some(tray) = app.tray_by_id(TRAY_ID) {
                let _ = tray.set_icon(if visible { icon.clone() } else { None });
            }
            visible = !visible;
            tokio::time::sleep(BLINK_INTERVAL).await;
        }

        // 只有「最后一个」任务负责恢复图标。被换代的旧任务直接走人，
        // 否则它的收尾会把新任务刚设好的空白帧覆盖掉。
        if BLINK_GEN.load(Ordering::SeqCst) == generation {
            if let Some(tray) = app.tray_by_id(TRAY_ID) {
                let _ = tray.set_icon(app.default_window_icon().cloned());
            }
        }
    });
}

/// 停止闪烁并恢复图标。用户看过主窗口就该停。
pub fn stop_blink() {
    BLINKING.store(false, Ordering::SeqCst);
}

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
