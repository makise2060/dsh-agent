mod cmd_ext;
mod env;
mod installer;
mod plugins;
mod process;
mod state;
mod version;

use state::AppState;
use tauri::Manager;

/// Tauri command registration — keeps the entry point declarative.
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_updater::Builder::new().build())
        .manage(AppState::default())
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { .. } = event {
                // Kill dsh process on window close (sync context — use try_lock)
                let app_state = window.app_handle().try_state::<AppState>();
                if let Some(s) = app_state {
                    // tokio::sync::Mutex::try_lock won't block
                    let pid = s.process_state.try_lock()
                        .map(|ps| ps.pid.unwrap_or(0))
                        .unwrap_or(0);
                    if pid > 0 {
                        #[cfg(target_os = "windows")]
                        {
                            use std::os::windows::process::CommandExt;
                            std::process::Command::new("taskkill")
                                .args(["/pid", &pid.to_string(), "/T", "/F"])
                                .creation_flags(0x08000000) // CREATE_NO_WINDOW
                                .spawn()
                                .ok();
                        }
                        #[cfg(not(target_os = "windows"))]
                        {
                            std::process::Command::new("kill")
                                .args(["-KILL", &pid.to_string()])
                                .spawn()
                                .ok();
                        }
                    }
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
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
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
