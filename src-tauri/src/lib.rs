mod bootstrap;
mod cmd_ext;
mod commands;
mod env;
mod installer;
mod logger;
mod plugins;
mod plugins_bundle;
mod process;
mod settings;
mod state;
mod tray;
mod version;
mod watcher;

use state::AppState;
use tauri::{Emitter, Manager};

/// 统一的退出入口：先杀掉 dsh 子进程树，再退出应用。
/// Job Object (KILL_ON_JOB_CLOSE) 兜住「主进程被强杀」这类异常路径。
pub fn quit(app: &tauri::AppHandle) {
    let app_state = app.state::<AppState>();
    let pid = app_state.get_dsh_pid();
    if pid > 0 {
        #[cfg(target_os = "windows")]
        {
            use std::os::windows::process::CommandExt;
            let _ = std::process::Command::new("taskkill")
                .args(["/pid", &pid.to_string(), "/T", "/F"])
                .creation_flags(0x08000000) // CREATE_NO_WINDOW
                .spawn();
        }
        #[cfg(not(target_os = "windows"))]
        {
            let _ = std::process::Command::new("kill")
                .args(["-KILL", &pid.to_string()])
                .spawn();
        }
    }
    app.exit(0);
}

/// 隐藏主窗口（最小化到托盘），dsh 子进程保持运行。
pub fn hide_main(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.hide();
    }
}

fn show_main(app: &tauri::AppHandle) {
    tray::show_main(app);
}

/// Tauri command registration — keeps the entry point declarative.
pub fn run() {
    // File logging (crashes, node command errors, connection errors)
    logger::init();

    let mut builder = tauri::Builder::default();

    // single-instance 必须最先注册，否则第二个实例可能已经跑完一部分初始化
    #[cfg(desktop)]
    {
        builder = builder.plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            show_main(app);
        }));
    }

    builder
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_notification::init())
        .manage(AppState::default())
        .manage(watcher::NotifyEnabled::default())
        .setup(|app| {
            // 托盘：关闭主窗口后应用驻留托盘，dsh 子进程保持运行
            if let Err(e) = tray::create(app.handle()) {
                log::error!("创建系统托盘失败: {}", e);
            }

            // 通知开关初值从 settings 灌入 —— managed state 的 Default 是
            // false，不读一次配置的话，用户上次开着的通知会在重启后静默失效
            app.state::<watcher::NotifyEnabled>()
                .set(settings::get_notify_on_done(app.handle()));

            // 防白屏的另一半：主窗口是 visible:false（见 tauri.conf.json），
            // 由前端首帧渲染后 show()。若前端坏了（构建损坏、资源缺失）永远
            // 不喊，这里 4 秒后强制亮出来 —— 宁可闪白，也不能让用户面对
            // 一个「不存在」的窗口。
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                tokio::time::sleep(std::time::Duration::from_secs(4)).await;
                if let Some(w) = handle.get_webview_window("main") {
                    if !w.is_visible().unwrap_or(true) {
                        log::info!("[app] 前端 4 秒未就绪，强制显示主窗口");
                        let _ = w.show();
                    }
                }
            });

            Ok(())
        })
        .on_window_event(|window, event| {
            // 用户把主窗口切回前台（Alt+Tab / 点任务栏），托盘不用再闪。
            if let tauri::WindowEvent::Focused(true) = event {
                if window.label() == "main" {
                    tray::stop_blink();
                }
                return;
            }

            let tauri::WindowEvent::CloseRequested { api, .. } = event else {
                return;
            };
            // 只拦主窗口
            if window.label() != "main" {
                return;
            }
            api.prevent_close();

            let app = window.app_handle().clone();
            // 记住的选择直接执行，不再打扰
            match settings::get_close_action(&app).as_deref() {
                Some("quit") => quit(&app),
                Some("tray") => hide_main(&app),
                // 没有记忆：通知前端弹关闭确认框
                _ => {
                    let _ = app.emit("close-requested", ());
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            // Bootstrap orchestration
            bootstrap::start_bootstrap,
            // Process management
            process::start_dsh,
            process::stop_dsh,
            process::get_dsh_status,
            process::restart_dsh,
            // Environment check
            env::check_environment,
            env::check_node_version,
            env::check_dsh_version,
            // Install / Update
            installer::install_dsh,
            version::check_dsh_update,
            version::check_app_update,
            // Plugin market
            plugins::search_plugins,
            plugins::list_installed_plugins,
            plugins::install_plugin,
            plugins::remove_plugin,
            plugins::activate_plugin,
            // Plugin bundle (dsh-web-ui all-in-one)
            plugins_bundle::check_bundle_status,
            plugins_bundle::install_bundle,
            plugins_bundle::verify_bundle,
            // Logging
            logger::get_logs_dir,
            // Close behavior
            commands::resolve_close,
            commands::get_close_action,
            commands::set_close_action,
            // Task-completion notification
            commands::get_notify_on_done,
            commands::set_notify_on_done,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

