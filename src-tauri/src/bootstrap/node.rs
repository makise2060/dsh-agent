//! Node.js 探测与版本闸门。

use std::path::PathBuf;

use semver::{Version, VersionReq};
use serde::Serialize;

/// dsh 及其插件声明的 engines：`^22.19.0 || >=24.0.0`
/// Node 23.x 不满足 `^22.19.0`（上界是 `<23.0.0`）。
/// 图省事写成 `>= 22.19` 会放行 23.x，故障表现是「装完插件宠物不出现」，
/// 从现象根本查不到根因。
const REQ_22: &str = "^22.19.0";
const REQ_24_PLUS: &str = ">=24.0.0";

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeInfo {
    /// node 可执行文件路径；系统 Node 时为 "node"
    pub path: PathBuf,
    pub version: String,
    /// 是应用自带的便携版还是用户系统里的
    pub portable: bool,
}

/// 版本是否满足 `^22.19.0 || >=24.0.0`
pub fn is_supported(v: &Version) -> bool {
    let r22 = VersionReq::parse(REQ_22).expect("硬编码的版本约束必须可解析");
    let r24 = VersionReq::parse(REQ_24_PLUS).expect("硬编码的版本约束必须可解析");
    r22.matches(v) || r24.matches(v)
}

/// 解析 `node --version` 的输出，形如 `v22.19.0`
pub fn parse_version(raw: &str) -> Option<Version> {
    Version::parse(raw.trim().trim_start_matches('v')).ok()
}

/// 探测系统 PATH 里的 Node。找不到或版本不合格都返回 None，
/// 由调用方决定是回落便携版还是报错。
pub fn detect_system_node() -> Option<NodeInfo> {
    let raw = run_stdout("node", &["--version"]).ok()?;
    let version = parse_version(&raw)?;
    if !is_supported(&version) {
        return None;
    }
    Some(NodeInfo {
        path: PathBuf::from("node"),
        version: version.to_string(),
        portable: false,
    })
}

/// 系统 Node 存在但版本不合格时，把版本号带出来用于提示
pub fn system_node_version() -> Option<String> {
    run_stdout("node", &["--version"])
        .ok()
        .and_then(|raw| parse_version(&raw))
        .map(|v| v.to_string())
}

fn run_stdout(program: &str, args: &[&str]) -> std::io::Result<String> {
    let output = super::dsh::sync_command(program)
        .args(args)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .output()?;
    if !output.status.success() {
        return Err(std::io::Error::other("非零退出码"));
    }
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}
