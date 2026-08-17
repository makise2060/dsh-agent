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
            log::info!("resolved real '{}' -> {}", name, p);
            return p.to_string();
        }
    }
    log::warn!("resolve_global_bin('{}') found no real npm binary, falling back to bare name", name);
    name.to_string()
}

#[cfg(not(target_os = "windows"))]
pub async fn resolve_global_bin(name: &str) -> String {
    name.to_string()
}

/// Windows Job Object with KILL_ON_JOB_CLOSE: every process assigned to it is
/// killed when the handle is closed — including when our own process dies
/// abruptly (crash / task manager kill), which never triggers CloseRequested.
/// This guarantees the dsh process tree cannot outlive dsh-agent.
#[cfg(target_os = "windows")]
pub struct JobGuard {
    handle: windows_sys::Win32::Foundation::HANDLE,
}

#[cfg(target_os = "windows")]
impl JobGuard {
    pub fn create() -> Option<JobGuard> {
        use std::mem::{size_of, zeroed};
        use windows_sys::Win32::Foundation::CloseHandle;
        use windows_sys::Win32::System::JobObjects::{
            CreateJobObjectW, SetInformationJobObject, JobObjectExtendedLimitInformation,
            JOBOBJECT_EXTENDED_LIMIT_INFORMATION, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
        };

        unsafe {
            let handle = CreateJobObjectW(std::ptr::null(), std::ptr::null());
            if handle.is_null() {
                log::warn!("CreateJobObjectW failed: {}", std::io::Error::last_os_error());
                return None;
            }
            let mut info: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = zeroed();
            info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
            let ok = SetInformationJobObject(
                handle,
                JobObjectExtendedLimitInformation,
                &info as *const _ as *const _,
                size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
            );
            if ok == 0 {
                log::warn!("SetInformationJobObject failed: {}", std::io::Error::last_os_error());
                CloseHandle(handle);
                return None;
            }
            Some(JobGuard { handle })
        }
    }

    /// Assign an already-spawned process (by pid) into the job.
    pub fn assign(&self, pid: u32) {
        use windows_sys::Win32::Foundation::CloseHandle;
        use windows_sys::Win32::System::JobObjects::AssignProcessToJobObject;
        use windows_sys::Win32::System::Threading::{
            OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_SET_QUOTA,
            PROCESS_TERMINATE,
        };

        unsafe {
            let proc = OpenProcess(
                PROCESS_SET_QUOTA | PROCESS_TERMINATE | PROCESS_QUERY_LIMITED_INFORMATION,
                0,
                pid,
            );
            if proc.is_null() {
                log::warn!("OpenProcess({}) failed: {}", pid, std::io::Error::last_os_error());
                return;
            }
            let ok = AssignProcessToJobObject(self.handle, proc);
            if ok == 0 {
                log::warn!(
                    "AssignProcessToJobObject({}) failed: {}",
                    pid,
                    std::io::Error::last_os_error()
                );
            }
            CloseHandle(proc);
        }
    }
}

#[cfg(target_os = "windows")]
impl Drop for JobGuard {
    fn drop(&mut self) {
        unsafe {
            windows_sys::Win32::Foundation::CloseHandle(self.handle);
        }
    }
}

// Windows handles are process-scoped and safe to move between threads.
#[cfg(target_os = "windows")]
unsafe impl Send for JobGuard {}

/// Log the launch context, then read one of dsh's profile plugin links.
///
/// The same `dsh-agent.exe` boots dsh fine from the Start menu but fails with
/// ERR_MODULE_NOT_FOUND when the installer's post-install checkbox launches it,
/// with the ~250 symlinks under `~/.dsh/profiles/node_modules` untouched on
/// disk the whole time. dsh reaches every profile plugin through those links,
/// and they were created by a non-elevated process — a process running with
/// RedirectionGuard in enforcing mode refuses to follow links it does not
/// trust, which looks exactly like a missing package. Inno Setup turns that
/// mitigation on for Setup itself ("RedirectionGuard status for current
/// process: Enabled in enforcing mode" in its log).
///
/// So: report the policy, and actually read through a link. Whichever way the
/// next installer run goes, the log says which of the two it was instead of
/// leaving us to guess again.
#[cfg(target_os = "windows")]
pub fn log_launch_context() {
    use windows_sys::Win32::System::Threading::{
        GetCurrentProcess, GetProcessMitigationPolicy, ProcessRedirectionTrustPolicy,
    };

    if let Ok(cwd) = std::env::current_dir() {
        log::info!("launch context: cwd={}", cwd.display());
    }

    unsafe {
        // windows-sys 0.59 ships the policy constant but not
        // PROCESS_MITIGATION_REDIRECTION_TRUST_POLICY. That type is a union of
        // a DWORD and a bitfield over the same 4 bytes, so a bare u32 is
        // byte-identical and needs no extra dependency.
        let mut flags: u32 = 0;
        let ok = GetProcessMitigationPolicy(
            GetCurrentProcess(),
            ProcessRedirectionTrustPolicy,
            &mut flags as *mut u32 as *mut core::ffi::c_void,
            std::mem::size_of::<u32>(),
        );
        if ok == 0 {
            log::warn!(
                "launch context: GetProcessMitigationPolicy failed: {}",
                std::io::Error::last_os_error()
            );
        } else {
            // Bit 0 = EnforceRedirectionTrust, bit 1 = AuditRedirectionTrust.
            log::info!(
                "launch context: RedirectionGuard enforce={} audit={} (raw=0x{:08x})",
                flags & 1,
                (flags >> 1) & 1,
                flags
            );
        }
    }

    probe_profile_link();
}

/// Read one profile plugin through its symlink. `@deepseek-ai/cordis-plugin-timer`
/// is the first entry dsh's loader reports as missing, so it is the one to check.
#[cfg(target_os = "windows")]
fn probe_profile_link() {
    let Some(home) = dirs::home_dir() else {
        log::warn!("launch context: no home dir, skipping profile link probe");
        return;
    };
    let link = home
        .join(".dsh")
        .join("profiles")
        .join("node_modules")
        .join("@deepseek-ai")
        .join("cordis-plugin-timer");

    match std::fs::symlink_metadata(&link) {
        Ok(m) => log::info!(
            "launch context: {} exists (symlink={})",
            link.display(),
            m.file_type().is_symlink()
        ),
        Err(e) => log::error!("launch context: symlink_metadata({}) failed: {}", link.display(), e),
    }

    // The decisive one: reading *through* the link is exactly what Node does
    // when it resolves the bare specifier.
    let pkg = link.join("package.json");
    match std::fs::read_to_string(&pkg) {
        Ok(s) => log::info!("launch context: read {} OK ({} bytes)", pkg.display(), s.len()),
        Err(e) => log::error!(
            "launch context: read {} FAILED: {} (kind={:?}, os={:?})",
            pkg.display(),
            e,
            e.kind(),
            e.raw_os_error()
        ),
    }
}
