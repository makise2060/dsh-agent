/// Platform helpers for spawning processes without visible console windows on Windows.
///
/// On Windows, `tokio::process::Command` (and `std::process::Command`) will
//  create a new console window for each child process by default.  This module
/// provides a wrapper that sets the `CREATE_NO_WINDOW` creation flag so that
/// no black cmd window flashes on screen.
use std::process::Stdio;

/// A thin wrapper around `tokio::process::Command` that automatically hides
/// the console window on Windows.
pub fn hidden_command(program: &str) -> tokio::process::Command {
    let mut cmd = tokio::process::Command::new(program);
    hide_on_windows(&mut cmd);
    cmd
}

/// Apply the `CREATE_NO_WINDOW` flag to a `tokio::process::Command` on Windows.
#[cfg(target_os = "windows")]
fn hide_on_windows(cmd: &mut tokio::process::Command) {
    #[allow(unused_imports)]
    use std::os::windows::process::CommandExt;
    // CREATE_NO_WINDOW = 0x08000000
    cmd.creation_flags(0x08000000);
}

#[cfg(not(target_os = "windows"))]
fn hide_on_windows(_cmd: &mut tokio::process::Command) {
    // no-op on non-Windows
}

/// Convenience: pipe stdout + stderr and hide window.
pub fn silent_command(program: &str) -> tokio::process::Command {
    let mut cmd = hidden_command(program);
    cmd.stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .stdin(Stdio::null());
    cmd
}

/// Resolve the real npm/npx executable path, skipping version-manager shims.
///
/// On Windows, tools like nvmd put shims (e.g. `~/.nvmd\bin\npx.exe`) ahead of
/// the real npm binaries in the PATH when the environment is rebuilt from the
/// registry (which is what happens for elevated processes launched by the
/// installer). Those shims hang in hidden/non-interactive contexts, so relying
/// on bare `npx`/`npm` names in `cmd /C` can stall forever.
///
/// We locate the real binary with `where <name>` and pick the first candidate
/// whose directory also contains `node_modules\npm` (the genuine npm install —
/// version-manager shim dirs have no such sibling). Falls back to the bare
/// name if resolution fails.
#[cfg(target_os = "windows")]
pub async fn resolve_global_bin(name: &str) -> String {
    let output = match hidden_command("cmd.exe")
        .args(["/C", "where", name])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()
        .await
    {
        Ok(o) if o.status.success() => o,
        _ => return name.to_string(),
    };
    let text = String::from_utf8_lossy(&output.stdout);
    for line in text.lines() {
        let p = line.trim();
        if p.is_empty() {
            continue;
        }
        let path = std::path::Path::new(p);
        let has_exec_ext = path
            .extension()
            .map(|e| e.eq_ignore_ascii_case("cmd") || e.eq_ignore_ascii_case("exe"))
            .unwrap_or(false);
        if !has_exec_ext {
            continue;
        }
        let has_real_npm = path
            .parent()
            .map(|d| d.join("node_modules").join("npm").is_dir())
            .unwrap_or(false);
        if has_real_npm {
            return p.to_string();
        }
    }
    name.to_string()
}

#[cfg(not(target_os = "windows"))]
pub async fn resolve_global_bin(name: &str) -> String {
    name.to_string()
}
