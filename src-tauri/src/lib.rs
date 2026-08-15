mod cmd_ext;
mod env;
mod installer;
mod logger;
mod plugins;
mod process;
mod state;
mod version;

use state::AppState;
use tauri::Manager;

/// Tauri command registration — keeps the entry point declarative.
pub fn run() {
    // File logging (crashes, node command errors, connection errors)
    logger::init();

    tauri::Builder::default()
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_opener::init())
        .manage(AppState::default())
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { .. } = event {
                // Kill the dsh process tree on window close. Reads the
                // lock-free pid mirror (not the async mutex) so a kill is
                // never skipped because the state lock is contended, and it
                // also covers the "Starting" window. If the app dies without
                // CloseRequested (crash / task manager), the Job Object
                // (KILL_ON_JOB_CLOSE) cleans the tree up for us.
                let app_state = window.app_handle().try_state::<AppState>();
                if let Some(s) = app_state {
                    let pid = s.get_dsh_pid();
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
            // Logging
            logger::get_logs_dir,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
