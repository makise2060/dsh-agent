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
