//! 界面插件全家桶：自动安装 + 装完自检。
//!
//! 聚合包 `@linxin666/dsh-web-ui-all`（鲸鱼娘、任务看板、皮肤中心、
//! better-sidebar 右侧面板等）。核心链路有四个真机踩过的坑：
//!
//! 1. **pnpm isolated 布局**：传递依赖收进 `.pnpm/`，而 dsh 按包名从 profile
//!    根解析 cordis 补丁 —— 必须 `nodeLinker: hoisted`，顶层见不到就等于
//!    没装好，`.pnpm/` 里找到也不算。
//! 2. **allowBuilds 白名单时机**：必须在 pnpm 第一次跑之前就位；事后补写会
//!    被「Already up to date」跳过，重试必须连 node_modules 一起清。
//! 3. **挂载凭据**：`dsh.profile.bundles` 由 dsh 在 pnpm 成功后才写 ——
//!    包在磁盘上 ≠ 插件已挂载，两处都要查。
//! 4. **pnpm-workspace.yaml 归 dsh 管**：只做增量补丁，绝不整体重写。
//!
//! 版本硬编码「已实测可用」而不是 latest：上游有过坏版本先例，用 latest 是
//! 在赌 registry。往上提版本前必须先真机实测一次。

use std::path::{Path, PathBuf};

use serde::Serialize;
use tauri::{AppHandle, Emitter};
use tokio::io::AsyncBufReadExt;

use crate::cmd_ext::hidden_command;
use crate::env::get_dsh_home;

/// 聚合包与硬编码的安装版本。0.2.x 起才包含 better-sidebar（右侧面板）。
pub const BUNDLE: &str = "@linxin666/dsh-web-ui-all";
pub const BUNDLE_VERSION: &str = "0.2.1";

/// 自检要确认落地的核心子包。
const REQUIRED_PACKAGES: &[&str] = &[
    "@linxin666/dsh-pet",
    "@linxin666/dsh-client-ui-task-board",
    "@linxin666/dsh-client-ui-skin-center",
    "dsh-better-sidebar",
    "@linxin666/dsh-skins",
];

/// 构建脚本裁决表（pnpm 11 `allowBuilds` 语义：true = 放行执行，
/// false = **明确拒绝、静默跳过** —— 不写才会炸 ERR_PNPM_IGNORED_BUILDS）。
/// 必须在 pnpm 第一次跑之前写好（坑 2）。
/// node-pty 是 0.2.x 的 dsh-ssh 新增依赖（真机实测 IGNORED_BUILDS 报出）。
const ALLOW_BUILDS: &[(&str, bool)] = &[
    ("cloudflared", true),
    ("cpu-features", false),
    ("node-pty", true),
    ("ssh2", true),
];

const PROFILE: &str = "web";

/// 硬编码快照：dsh 0.1.0-rc.6 scaffold 的逐字抄本，只含上游自己的键。
/// 只在 dsh 初始化没产出文件时兜底；所需键全走增量补丁。
const SCAFFOLD: &str = "packages:\n  - .\n\nnodeLinker: hoisted\nautoInstallPeers: false\n";

/// npm 镜像降级顺序：国内直连 registry.npmjs.org 经常超时，镜像优先。
const NPM_REGISTRIES: &[&str] = &[
    "https://registry.npmmirror.com",
    "https://registry.npmjs.org",
];

/// pnpm 拒绝执行依赖构建脚本时的特征错误。
const IGNORED_BUILDS: &str = "ERR_PNPM_IGNORED_BUILDS";

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "snake_case")]
pub struct BundleStatus {
    pub status: String, // "not_installed" | "installed" | "needs_repair"
    pub installed_version: Option<String>,
    pub expected_version: String,
    pub warning: Option<String>,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "snake_case")]
struct BundleProgress {
    package: String,
    stage: String, // starting | progress | done | error | retry
    message: String,
    percent: Option<u32>,
}

fn emit_progress(app: &AppHandle, stage: &str, message: impl Into<String>, percent: Option<u32>) {
    let _ = app.emit(
        "plugin-install-progress",
        BundleProgress {
            package: BUNDLE.to_string(),
            stage: stage.to_string(),
            message: message.into(),
            percent,
        },
    );
}

// ── 路径助手 ────────────────────────────────────────────────────

fn profile_dir() -> PathBuf {
    get_dsh_home().join("profiles").join(PROFILE)
}

fn workspace_yaml() -> PathBuf {
    profile_dir().join("pnpm-workspace.yaml")
}

fn node_modules_dir() -> PathBuf {
    profile_dir().join("node_modules")
}

fn profile_package_json() -> PathBuf {
    profile_dir().join("package.json")
}

/// 顶层路径 `node_modules/@scope/name`。
fn top_level(modules: &Path, pkg: &str) -> PathBuf {
    pkg.split('/')
        .fold(modules.to_path_buf(), |acc, seg| acc.join(seg))
}

/// 包只存在于 pnpm 的 isolated 仓库里时，返回它的真实位置。
/// 目录名是把 `/` 换成 `+` 再接 `@版本`，版本不预设、按前缀扫。
fn in_pnpm_store(modules: &Path, pkg: &str) -> Option<PathBuf> {
    let prefix = format!("{}@", pkg.replace('/', "+"));
    for entry in std::fs::read_dir(modules.join(".pnpm")).ok()?.flatten() {
        if !entry.file_name().to_string_lossy().starts_with(&prefix) {
            continue;
        }
        let nested = top_level(&entry.path().join("node_modules"), pkg);
        if nested.join("package.json").is_file() {
            return Some(nested);
        }
    }
    None
}

/// 定位一个包，顶层找不到就去 `.pnpm/` 里找。
/// 只用于读版本号这类信息性用途，不能拿来判断插件可不可用。
fn resolve_package(modules: &Path, pkg: &str) -> Option<PathBuf> {
    let direct = top_level(modules, pkg);
    if direct.join("package.json").is_file() {
        return Some(direct);
    }
    in_pnpm_store(modules, pkg)
}

/// 包是否真的能被 dsh 加载。
///
/// **只认 `node_modules/` 顶层**（坑 1）—— `.pnpm/` 里找到也判不通过。
/// 判据取自包的实际声明：`cordis.patch.yml`（由 `dsh.bundle.patch` 声明）
/// 必须在；声明了 `main` 就必须真的存在；纯聚合包没有 main，不强求。
fn package_is_usable(modules: &Path, pkg: &str) -> bool {
    let dir = top_level(modules, pkg);
    let Ok(text) = std::fs::read_to_string(dir.join("package.json")) else {
        return false;
    };
    let Ok(json) = serde_json::from_str::<serde_json::Value>(&text) else {
        return false;
    };

    let patch = json
        .pointer("/dsh/bundle/patch")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("cordis.patch.yml");
    if !dir.join(patch.trim_start_matches("./")).is_file() {
        return false;
    }

    match json.get("main").and_then(serde_json::Value::as_str) {
        Some(main) => dir.join(main).is_file(),
        None => true,
    }
}

/// 聚合包是否真的挂到了 profile 上（坑 3）。
/// `dependencies` 是 pnpm 写的，`dsh.profile.bundles` 才是 dsh 写的挂载凭据。
fn is_mounted() -> bool {
    let Ok(text) = std::fs::read_to_string(profile_package_json()) else {
        return false;
    };
    let Ok(json) = serde_json::from_str::<serde_json::Value>(&text) else {
        return false;
    };
    json.pointer("/dsh/profile/bundles")
        .and_then(serde_json::Value::as_array)
        .is_some_and(|list| list.iter().any(|v| v.as_str() == Some(BUNDLE)))
}

/// 已装的聚合包版本。没装返回 None。
fn installed_version() -> Option<String> {
    let modules = node_modules_dir();
    let dir = resolve_package(&modules, BUNDLE)?;
    let text = std::fs::read_to_string(dir.join("package.json")).ok()?;
    let json: serde_json::Value = serde_json::from_str(&text).ok()?;
    Some(json.get("version")?.as_str()?.to_string())
}

// ── pnpm-workspace.yaml 增量补丁（行级文本处理，绝不整体重写）─────

/// 幂等地补齐 allowBuilds 裁决。
///
/// 两种缺口都要处理：
/// - 块内没有该键 → 追加 `  <pkg>: <bool>`
/// - 块内有该键但值是 dsh 写的占位符 `set this to true or false` → 覆盖为真值
///
/// 已是 true/false 的存量裁决一律不动（含旧版本写下的 cpu-features: true）。
fn patch_allow_builds(src: &str) -> String {
    let mut lines: Vec<String> = src.lines().map(str::to_string).collect();
    let mut in_block = false;
    let mut block_start: Option<usize> = None;
    let mut block_end = lines.len();

    // 先定位 allowBuilds 块的范围
    for (i, line) in lines.iter().enumerate() {
        if line.trim_start() == "allowBuilds:" {
            in_block = true;
            block_start = Some(i);
            continue;
        }
        if in_block {
            if !line.starts_with(char::is_whitespace) && !line.trim().is_empty() {
                block_end = i;
                break;
            }
            block_end = i + 1;
        }
    }

    // 块内已有的键：覆盖占位符值
    if let Some(start) = block_start {
        for i in (start + 1)..block_end.min(lines.len()) {
            let (key, val) = match lines[i].trim().split_once(':') {
                Some((k, v)) => (
                    k.trim().trim_matches('\'').trim_matches('"').to_string(),
                    v.trim().to_string(),
                ),
                None => continue,
            };
            let Some((pkg, allow)) = ALLOW_BUILDS.iter().find(|(p, _)| *p == key) else {
                continue;
            };
            if val != "true" && val != "false" {
                let indent: String = lines[i]
                    .chars()
                    .take_while(|c| c.is_whitespace())
                    .collect();
                lines[i] = format!("{indent}{pkg}: {allow}");
            }
        }
    }

    // 块内还没有的键：插到块首（allowBuilds: 下一行），保持缩进块连续
    let still_missing: Vec<&(&str, bool)> = ALLOW_BUILDS
        .iter()
        .filter(|(pkg, _)| {
            let seg = match block_start {
                Some(start) => &lines[(start + 1)..block_end.min(lines.len())],
                None => &lines[..],
            };
            !seg.iter().any(|l| {
                l.trim()
                    .split_once(':')
                    .map(|(k, v)| {
                        k.trim().trim_matches('\'').trim_matches('"') == *pkg
                            && (v.trim() == "true" || v.trim() == "false")
                    })
                    .unwrap_or(false)
            })
        })
        .collect();

    if still_missing.is_empty() {
        return lines.join("\n");
    }

    match block_start {
        Some(start) => {
            let mut insert: Vec<String> = Vec::new();
            for (pkg, allow) in still_missing {
                insert.push(format!("  {pkg}: {allow}"));
            }
            // 插到 allowBuilds: 之后（保持连续缩进块）
            lines.splice((start + 1)..(start + 1), insert);
            lines.join("\n")
        }
        None => {
            let mut out = src.to_string();
            if !out.is_empty() && !out.ends_with('\n') {
                out.push('\n');
            }
            out.push_str("allowBuilds:\n");
            for (pkg, allow) in ALLOW_BUILDS {
                out.push_str(&format!("  {pkg}: {allow}\n"));
            }
            out
        }
    }
}

/// `minimumReleaseAgeExclude` 块里是否已有整 scope 的通配条目。
/// 逐包条目（`- '@linxin666/dsh-pet@0.1.15'`）不算数 —— 那些挡不住
/// 「下一个还没被记录的新版本」。
fn has_release_age_wildcard(src: &str) -> bool {
    let mut in_block = false;
    for line in src.lines() {
        if line.trim_start().starts_with("minimumReleaseAgeExclude:") {
            in_block = true;
            continue;
        }
        if in_block {
            if !line.starts_with(char::is_whitespace) && !line.trim().is_empty() {
                return false;
            }
            let item = line
                .trim()
                .trim_start_matches('-')
                .trim()
                .trim_matches('\'')
                .trim_matches('"');
            if item == "@linxin666/*" {
                return true;
            }
        }
    }
    false
}

/// 幂等地补上 `@linxin666/*` 的 release-age 排除
/// （官方 README：新版发布后约 10 天内 pnpm 11 可能静默装回旧版并写坏配置）。
fn patch_release_age(src: &str) -> String {
    if has_release_age_wildcard(src) {
        return src.to_string();
    }

    let added = "  - '@linxin666/*'\n";

    match src
        .lines()
        .position(|l| l.trim_start().starts_with("minimumReleaseAgeExclude:"))
    {
        Some(idx) => {
            let mut out = String::new();
            for (i, line) in src.lines().enumerate() {
                out.push_str(line);
                out.push('\n');
                if i == idx {
                    out.push_str(added);
                }
            }
            out
        }
        None => {
            let mut out = src.to_string();
            if !out.is_empty() && !out.ends_with('\n') {
                out.push('\n');
            }
            out.push_str("minimumReleaseAgeExclude:\n");
            out.push_str(added);
            out
        }
    }
}

/// 幂等补上我们必需的两段配置：allowBuilds（坑 2）与 release-age 排除。
/// 对 dsh 生成的文件与硬编码快照一视同仁，存量安装下次也会被补齐。
fn ensure_allow_builds() -> Result<(), String> {
    let path = workspace_yaml();
    let dir = profile_dir();
    std::fs::create_dir_all(&dir).map_err(|e| format!("创建 profile 目录失败: {e}"))?;

    let original = match std::fs::read_to_string(&path) {
        Ok(text) => text,
        Err(_) => {
            log::info!(
                "[bundle] dsh 未生成 pnpm-workspace.yaml，使用硬编码快照（抄自 dsh 0.1.0-rc.6 实际输出）"
            );
            SCAFFOLD.to_string()
        }
    };

    let patched = patch_release_age(&patch_allow_builds(&original));
    if patched != original {
        std::fs::write(&path, patched).map_err(|e| format!("写入 {} 失败: {e}", path.display()))?;
    }
    Ok(())
}

/// 先跑一条只读命令触发 dsh 自己初始化 profile —— 它建的文件永远是
/// 它当前版本想要的样子。失败无所谓：`plugin add` 那步反正也会初始化。
async fn ensure_profile_scaffold() {
    if workspace_yaml().is_file() {
        return;
    }
    log::info!("[bundle] 先让 dsh 自行初始化 profile（避免动用硬编码快照）");
    #[cfg(target_os = "windows")]
    {
        let npx = crate::cmd_ext::resolve_global_bin("npx").await;
        let _ = hidden_command("cmd.exe")
            .args(["/C", npx.as_str(), "@deepseek-ai/dsh", "--profile", PROFILE, "--dump-config"])
            .output()
            .await;
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = hidden_command("npx")
            .args(["@deepseek-ai/dsh", "--profile", PROFILE, "--dump-config"])
            .output()
            .await;
    }
}

// ── 自检判定链 ──────────────────────────────────────────────────

/// 完整自检。返回 Ok(Some(警告)) = 能用但有疑点；只有聚合包整体缺失才 Err。
fn verify_impl() -> Result<Option<String>, String> {
    let modules = node_modules_dir();

    // 聚合包都找不到 = 安装是真没发生
    if resolve_package(&modules, BUNDLE).is_none() {
        return Err(format!("{BUNDLE} 没有落地，界面插件全部不可用。"));
    }

    // 包在、但没写进 dsh.profile.bundles —— 装了却没挂上
    if !is_mounted() {
        return Ok(Some(
            "界面插件已下载但没有挂载到 profile（dsh 在写 bundles 前就失败了），\
             所以鲸鱼娘、皮肤中心等不会出现。点「修复安装」可以补上。"
                .into(),
        ));
    }

    let missing: Vec<&str> = REQUIRED_PACKAGES
        .iter()
        .copied()
        .filter(|pkg| !package_is_usable(&modules, pkg))
        .collect();

    if missing.is_empty() {
        return Ok(None);
    }

    // 顶层没有、`.pnpm/` 里却有 —— 不是没装，是 pnpm 用错了布局。
    let stashed: Vec<&str> = missing
        .iter()
        .copied()
        .filter(|pkg| in_pnpm_store(&modules, pkg).is_some())
        .collect();

    if !stashed.is_empty() {
        return Ok(Some(format!(
            "插件已下载但摆放位置不对（{}），dsh 加载不到。\
             删除 ~/.dsh/profiles/web 整个目录后重启即可重装。",
            stashed.join("、")
        )));
    }

    log::warn!("[bundle] 未确认落地的子包：{}", missing.join("、"));
    Ok(Some(format!(
        "以下界面插件未能确认挂载：{}。若鲸鱼娘等功能没出现，可点「修复安装」重装。",
        missing.join("、")
    )))
}

fn bundle_status_from(result: Result<Option<String>, String>) -> BundleStatus {
    let installed_version = installed_version();
    match result {
        Ok(None) => BundleStatus {
            status: "installed".into(),
            installed_version,
            expected_version: BUNDLE_VERSION.into(),
            warning: None,
        },
        Ok(Some(warning)) => BundleStatus {
            status: "needs_repair".into(),
            installed_version,
            expected_version: BUNDLE_VERSION.into(),
            warning: Some(warning),
        },
        Err(e) => BundleStatus {
            status: "needs_repair".into(),
            installed_version,
            expected_version: BUNDLE_VERSION.into(),
            warning: Some(e),
        },
    }
}

// ── Tauri 命令 ──────────────────────────────────────────────────

/// 检测全家桶安装状态。纯文件读取，不拉起 node，可安全在启动时静默调用。
#[tauri::command]
pub async fn check_bundle_status() -> Result<BundleStatus, String> {
    // dependencies 里没有聚合包 = 从未安装
    let installed_names = crate::plugins::get_installed_package_names()
        .await
        .unwrap_or_default();
    if !installed_names.iter().any(|n| n == BUNDLE) {
        return Ok(BundleStatus {
            status: "not_installed".into(),
            installed_version: None,
            expected_version: BUNDLE_VERSION.into(),
            warning: None,
        });
    }
    Ok(bundle_status_from(verify_impl()))
}

/// 安装界面插件全家桶（含 pnpm-workspace 补丁、registry 降级、构建白名单重试）。
#[tauri::command]
pub async fn install_bundle(app: AppHandle) -> Result<BundleStatus, String> {
    emit_progress(&app, "starting", "正在安装界面插件全家桶…", None);

    // 先给 dsh 机会自己建 profile，再打我们的增量补丁
    ensure_profile_scaffold().await;
    ensure_allow_builds()?;

    let mut attempt = 0u32;
    loop {
        let mut last_err: Option<String> = None;

        for registry in NPM_REGISTRIES {
            log::info!("[bundle] 安装 {BUNDLE}@{BUNDLE_VERSION}（registry: {registry}）");
            match run_add(&app, registry).await {
                Ok(()) => {
                    emit_progress(&app, "done", "界面插件安装完成", Some(100));
                    return Ok(bundle_status_from(verify_impl()));
                }
                Err(e) => {
                    if e.contains(IGNORED_BUILDS) {
                        log::warn!("[bundle] 撞到 ERR_PNPM_IGNORED_BUILDS，补写 allowBuilds 后重试");
                        // 坑 2：只补配置会被 pnpm「Already up to date」跳过，必须连 node_modules 一起清
                        ensure_allow_builds()?;
                        if let Err(e) = std::fs::remove_dir_all(node_modules_dir()) {
                            log::warn!("[bundle] node_modules 清理失败（仍继续重试）：{e}");
                        }
                        return Err(e);
                    }
                    last_err = Some(e);
                }
            }
        }

        // 所有 registry 都失败：如果之前没重试过且疑似白名单问题，再给一次机会
        attempt += 1;
        if attempt < 2 {
            log::warn!("[bundle] 所有 registry 安装失败，补写 allowBuilds 并清空 node_modules 后重试");
            ensure_allow_builds()?;
            if let Err(e) = std::fs::remove_dir_all(node_modules_dir()) {
                log::warn!("[bundle] node_modules 清理失败（仍继续重试）：{e}");
            }
            continue;
        }

        let err = last_err.unwrap_or_else(|| "没有可用的 npm registry".into());
        emit_progress(&app, "error", &err, None);
        return Err(err);
    }
}

/// 跑一次 `dsh plugin add`，registry 走镜像降级。
async fn run_add(app: &AppHandle, registry: &str) -> Result<(), String> {
    let spec = format!("{BUNDLE}@{BUNDLE_VERSION}");

    #[cfg(target_os = "windows")]
    let mut child = {
        let npx = crate::cmd_ext::resolve_global_bin("npx").await;
        hidden_command("cmd.exe")
            .args([
                "/C",
                npx.as_str(),
                "@deepseek-ai/dsh",
                "plugin",
                "--profile",
                PROFILE,
                "add",
                &spec,
            ])
            .env("npm_config_registry", registry)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| format!("Failed to run dsh plugin add: {e}"))?
    };

    #[cfg(not(target_os = "windows"))]
    let mut child = hidden_command("npx")
        .args([
            "@deepseek-ai/dsh",
            "plugin",
            "--profile",
            PROFILE,
            "add",
            &spec,
        ])
        .env("npm_config_registry", registry)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to run dsh plugin add: {e}"))?;

    let stdout = child.stdout.take().ok_or("No stdout")?;
    let stderr = child.stderr.take().ok_or("No stderr")?;
    let app_for_stdout = app.clone();

    // stdout：实时进度（pnpm 的 Progress 行 → 百分比）
    let stdout_task = tauri::async_runtime::spawn(async move {
        let reader = tokio::io::BufReader::new(stdout);
        let mut lines = reader.lines();
        while let Ok(Some(line)) = lines.next_line().await {
            let trimmed = line.trim();
            log::info!("[bundle] {}", trimmed);
            if let Some((text, percent)) = install_progress(trimmed) {
                emit_progress(&app_for_stdout, "progress", text, percent);
            } else {
                emit_progress(&app_for_stdout, "progress", trimmed, None);
            }
        }
    });

    // stderr：累积到最后判断 IGNORED_BUILDS / 其他错误
    let mut stderr_text = String::new();
    {
        let reader = tokio::io::BufReader::new(stderr);
        let mut lines = reader.lines();
        while let Ok(Some(line)) = lines.next_line().await {
            log::info!("[bundle:err] {}", line);
            stderr_text.push_str(&line);
            stderr_text.push('\n');
        }
    }

    let status = child.wait().await.map_err(|e| e.to_string())?;
    let _ = stdout_task.await;
    if status.success() {
        Ok(())
    } else if stderr_text.contains(IGNORED_BUILDS) {
        Err(IGNORED_BUILDS.to_string())
    } else {
        let tail: Vec<&str> = stderr_text.lines().rev().take(10).collect();
        Err(format!(
            "插件安装失败 (exit code: {})\n--- 最近输出 ---\n{}",
            status.code().unwrap_or(-1),
            tail.join("\n")
        ))
    }
}

/// 把 pnpm 的一行输出翻译成「当前活动」+ 完成比例。
/// `Progress: resolved 31, reused 30, downloaded 0, added 16`
fn install_progress(line: &str) -> Option<(String, Option<u32>)> {
    if line.contains("Verifying lockfile") {
        return Some(("正在校验依赖来源…".into(), None));
    }
    let rest = line.strip_prefix("Progress:")?;
    let field = |name: &str| -> Option<u32> {
        rest.split(',')
            .find_map(|part| part.trim().strip_prefix(name)?.trim().parse().ok())
    };
    let total = field("resolved")?;
    let done = field("added").unwrap_or(0);
    let percent = (total > 0).then(|| ((done as f64 / total as f64) * 100.0) as u32).map(|p| p.min(99));
    Some((format!("正在安装界面插件… {done}/{total}"), percent))
}

/// 手动触发自检。
#[tauri::command]
pub async fn verify_bundle(app: AppHandle) -> Result<BundleStatus, String> {
    let status = bundle_status_from(verify_impl());
    if status.status == "installed" {
        emit_progress(&app, "done", "自检通过：界面插件全家桶状态正常", Some(100));
    } else {
        emit_progress(
            &app,
            "error",
            status.warning.clone().unwrap_or_else(|| "自检未通过".into()),
            None,
        );
    }
    Ok(status)
}

/// 引导编排用：返回装完自检的警告（Ok(Some) = 能用但有疑点）。
pub async fn verify_impl_public() -> Result<Option<String>, String> {
    verify_impl()
}
