//! dsh 的定位、安装与 pnpm 准备。

use std::path::PathBuf;

use super::mirror::NPM_REGISTRIES;
use super::Reporter;
use crate::bootstrap::Stage;

pub const PACKAGE: &str = "@deepseek-ai/dsh";

/// 构造一个不弹控制台窗口的同步 Command（引导流程在 spawn_blocking 中运行）。
/// 与 cmd_ext::hidden_command（tokio 版）等价，但用 std::process::Command。
pub(crate) fn sync_command(program: &str) -> std::process::Command {
    let mut cmd = std::process::Command::new(program);
    cmd.stdin(std::process::Stdio::null());
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
    }
    cmd
}

/// npm 自身是 `.cmd`，只能通过 cmd /C 调用（sync_command 已带 CREATE_NO_WINDOW）。
/// 走 run_checked：它会把「执行了什么」与失败时 stdout+stderr 的完整输出落进日志。
fn npm(args: &[&str]) -> Result<String, String> {
    let line = format!("npm {}", args.join(" "));
    let out = run_checked("cmd", &["/C", &line])?;
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

/// 跑一条命令并要求它成功退出，否则带上 stderr 报错。
pub fn run_checked(program: &str, args: &[&str]) -> Result<std::process::Output, String> {
    let display = format!("{program} {}", args.join(" "));
    log::info!("[proc] 执行：{display}");

    let output = sync_command(program)
        .args(args)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .output()
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                format!("找不到可执行文件：{program}")
            } else {
                format!("执行失败：{e}")
            }
        })?;

    if !output.status.success() {
        let code = output.status.code().unwrap_or(-1);
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        log::error!("[proc] 失败（退出码 {code}）：{display}\n{stderr}");
        return Err(if stderr.trim().is_empty() {
            format!("命令失败（退出码 {code}）：{display}")
        } else {
            stderr
        });
    }
    Ok(output)
}

/// 全局 node_modules 根目录，形如 `C:\Users\x\AppData\Roaming\npm\node_modules`
fn npm_root_global() -> Result<PathBuf, String> {
    let raw = npm(&["root", "-g"])?;
    let path = PathBuf::from(raw.trim());
    if path.is_dir() {
        Ok(path)
    } else {
        Err(format!("npm root -g 返回了不存在的路径：{}", path.display()))
    }
}

/// dsh 的 JS 入口。未安装时返回 None。
pub fn entry_point() -> Result<Option<PathBuf>, String> {
    let root = npm_root_global()?;
    let entry = root.join("@deepseek-ai").join("dsh").join("lib").join("bin.js");
    Ok(entry.is_file().then_some(entry))
}

/// 已安装 dsh 的版本（读 package.json，不启动子进程问 --version）。
pub fn installed_version(entry: &PathBuf) -> Option<String> {
    let pkg = entry.parent()?.parent()?.parent()?.join("package.json");
    let text = std::fs::read_to_string(pkg).ok()?;
    let json: serde_json::Value = serde_json::from_str(&text).ok()?;
    json.get("version")?.as_str().map(str::to_string)
}

/// dsh 装插件时会调用 `pnpm`，干净机器上没有就会失败在
/// 「'pnpm' 不是内部或外部命令」上。这里先检查再补装。
pub fn pnpm_available() -> bool {
    sync_command("cmd")
        .args(["/C", "pnpm --version"])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .output()
        .is_ok_and(|out| out.status.success())
}

/// 全局安装 pnpm。镜像源优先，失败回落官方。
pub fn install_pnpm() -> Result<(), String> {
    let mut last: Option<String> = None;
    for registry in NPM_REGISTRIES {
        log::info!("[pnpm] 通过 {} 安装", registry.name);
        match npm(&[
            "install",
            "-g",
            "pnpm",
            "--registry",
            registry.base,
            "--no-fund",
            "--no-audit",
        ]) {
            Ok(_) => return Ok(()),
            Err(e) => {
                log::warn!("[pnpm] registry {} 安装失败: {e}", registry.name);
                last = Some(e);
            }
        }
    }
    Err(last.unwrap_or_else(|| "没有可用的 npm registry".into()))
}

/// 全局安装 dsh。镜像源优先，失败回落官方。
pub fn install(reporter: &Reporter) -> Result<PathBuf, String> {
    let mut last: Option<String> = None;

    for registry in NPM_REGISTRIES {
        reporter.detail(Stage::InstallingDsh, format!("通过 {} 安装（首次约需 1-3 分钟）", registry.name));
        // 只在本次命令上临时指定 registry，不动用户的全局 npm 配置
        match npm(&[
            "install",
            "-g",
            PACKAGE,
            "--registry",
            registry.base,
            "--no-fund",
            "--no-audit",
        ]) {
            Ok(_) => {
                return entry_point()?.ok_or_else(|| {
                    "npm 报告安装成功，但找不到 dsh 入口文件".into()
                })
            }
            Err(e) => {
                log::warn!("[dsh] registry {} 安装失败: {e}", registry.name);
                last = Some(e);
            }
        }
    }

    Err(last.unwrap_or_else(|| "没有可用的 npm registry".into()))
}
