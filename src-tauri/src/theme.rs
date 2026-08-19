//! 主题联动：外壳外观与 dsh web 的 `ui-theme.preference` 双向同步。
//!
//! dsh 的外观设置是唯一真值（存于 `$DSH_HOME/settings.yaml` 的
//! `ui-theme.preference`，取值 light / dark / system）。外壳跟随它，
//! 用户在外壳切换时写回它 —— 页面实时翻转、下次启动也生效。

use serde::Serialize;
use tauri::{AppHandle, Emitter};

pub const EVENT_THEME: &str = "app:theme";

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ThemePayload {
    /// 写入后的偏好值：light / dark / system
    pub preference: String,
}

fn settings_path() -> Option<std::path::PathBuf> {
    Some(crate::env::get_dsh_home().join("settings.yaml"))
}

/// 取 `ui-theme.preference` 的原始值。手写解析而不是引 YAML 库：
/// 只取一个固定位置的标量。文件缺失或没有该段时返回 "system"（默认跟随系统）。
fn read_preference(text: &str) -> Option<String> {
    let mut in_block = false;
    for line in text.lines() {
        // 顶层键（无缩进）决定进入或离开 ui-theme 段
        if !line.starts_with([' ', '\t']) {
            in_block = line.trim_end() == "ui-theme:";
            continue;
        }
        if !in_block {
            continue;
        }
        if let Some(v) = line.trim().strip_prefix("preference:") {
            return Some(v.trim().trim_matches(['"', '\'']).to_string());
        }
    }
    None
}

/// 读当前偏好：light / dark / system（缺失默认 system）。
#[tauri::command]
pub fn get_theme_preference() -> String {
    let Some(path) = settings_path() else {
        return "system".into();
    };
    std::fs::read_to_string(path)
        .ok()
        .and_then(|t| read_preference(&t))
        .filter(|v| v == "light" || v == "dark" || v == "system")
        .unwrap_or_else(|| "system".into())
}

/// 写 `ui-theme.preference` 并广播事件。
///
/// 行级编辑，只动目标键，不重排文件 —— settings.yaml 归 dsh 管，
/// YAML 解析重排会弄砸别人的配置。文件缺失时新建最小结构。
#[tauri::command]
pub fn set_theme_preference(app: AppHandle, preference: String) -> Result<String, String> {
    let preference = if preference == "light" || preference == "dark" || preference == "system" {
        preference
    } else {
        return Err(format!("非法主题偏好：{preference}"));
    };

    let Some(path) = settings_path() else {
        return Err("无法定位 DSH_HOME".into());
    };
    // 文件缺失：建最小结构
    if !path.exists() {
        std::fs::write(&path, "ui-theme:\n  preference: system\n")
            .map_err(|e| format!("创建 settings.yaml 失败: {e}"))?;
    }

    let text = std::fs::read_to_string(&path).map_err(|e| format!("读取 settings.yaml 失败: {e}"))?;
    let patched = patch_preference(&text, &preference);
    std::fs::write(&path, patched).map_err(|e| format!("写入 settings.yaml 失败: {e}"))?;

    let _ = app.emit(EVENT_THEME, ThemePayload { preference: preference.clone() });
    Ok(preference)
}

/// 行级替换 `ui-theme.preference`。段不存在则追加。
fn patch_preference(src: &str, value: &str) -> String {
    let mut lines: Vec<String> = src.lines().map(str::to_string).collect();
    let mut in_block = false;
    let mut patched = false;

    for line in lines.iter_mut() {
        if !line.starts_with([' ', '\t']) {
            in_block = line.trim_end() == "ui-theme:";
            continue;
        }
        if in_block && line.trim().starts_with("preference:") {
            *line = format!("  preference: {}", value);
            patched = true;
            break;
        }
    }

    if !patched {
        // 段存在但没 preference 键：在段内补；段不存在：整体追加
        let has_block = lines.iter().any(|l| l.trim_end() == "ui-theme:");
        if has_block {
            // 找段内第一行缩进，插在段首
            if let Some(idx) = lines.iter().position(|l| l.trim_end() == "ui-theme:") {
                lines.insert(idx + 1, format!("  preference: {}", value));
            }
        } else {
            if !lines.is_empty() && !lines.last().map(|l| l.is_empty()).unwrap_or(false) {
                lines.push(String::new());
            }
            lines.push("ui-theme:".into());
            lines.push(format!("  preference: {}", value));
        }
    }

    lines.join("\n")
}
