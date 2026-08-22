//! profile 插件挂载冲突的预防与自愈。
//!
//! 真机日志（2026-08-19，v1.3.0 首日用户）：升级后 dsh 启动即崩、三次重试
//! 全灭 ——
//!
//! ```text
//! Error: dsh: plugin tree failed to load: failed to apply loader entry
//! web-ui-better-sidebar (dsh-better-sidebar):
//! webserver: duplicate prefix route "/sidebar/api"
//! ```
//!
//! 根因：聚合包 0.2.x 起内置了 better-sidebar，而从旧版升上来的 profile 里
//! 往往还挂着单独安装的同名插件（`dsh plugin add` 会把它写进
//! `dsh.profile.bundles` 或 `cordis.patch.yml` 作为挂载凭据）。两个 loader
//! entry 各自 apply 一遍 `dsh-better-sidebar`，第二个向 webserver 注册
//! `/sidebar/api` 前缀路由时撞车。dsh 对此零容忍：整棵插件树加载失败、
//! 进程退出，且每次启动必然复现 —— 用户除了手删整个 profile 无路可走。
//!
//! 封装层在此做两层防御：
//!
//! 1. **预防**（[`preflight_cleanup`]）：确认聚合包挂载健在后，摘除指向其
//!    子包的存量独立挂载。聚合包是我们受控安装的那份，留它、摘别的。
//! 2. **自愈**（[`repair_duplicate_route_failure`]）：dsh 启动失败且报
//!    duplicate prefix route 时，从错误文本解析出冲突包名，摘除它的独立
//!    挂载，让下一轮重试能过。将来聚合包换子包、出现新的撞名组合也不用改代码。
//!
//! 只清「挂载凭据」（bundles 数组、patch 条目）。dependencies 里的残留包
//! 不产生 loader entry、不影响启动，留给 pnpm 下次操作自然收敛。

use crate::env::get_dsh_home;
use crate::plugins_bundle::{self, BUNDLE};

/// dsh 报插件重复注册前缀路由时的特征片段。
const DUP_ROUTE_MARK: &str = "duplicate prefix route";

/// 从启动失败的错误文本中解析出撞路由的插件包名。
///
/// 匹配 `loader entry <entry> (<pkg>)`，且该行必须同时含有
/// [`DUP_ROUTE_MARK`] —— 两个标记分属两行的错误更可能是别的问题，宁可不修。
pub fn extract_conflicting_package(err: &str) -> Option<String> {
    let re = regex::Regex::new(r"loader entry (\S+) \(([^)]+)\)").ok()?;
    err.lines()
        .filter(|line| line.contains(DUP_ROUTE_MARK))
        .find_map(|line| re.captures(line).map(|caps| caps[2].to_string()))
}

// ── 挂载凭据的纯函数摘除（无 IO，便于单测）──────────────────────

/// 从 package.json 文本中摘除 `dsh.profile.bundles` 里的 pkg。
/// 返回 `Some((新文本, 是否有改动))`；JSON 无法解析时返回 None（绝不动文件）。
/// serde_json 会把键序规范成字母序 —— 该文件本就由 pnpm/dsh 机器管理，无碍。
fn strip_from_bundles(text: &str, pkg: &str) -> Option<(String, bool)> {
    let mut json: serde_json::Value = serde_json::from_str(text).ok()?;
    let Some(bundles) = json
        .pointer_mut("/dsh/profile/bundles")
        .and_then(|v| v.as_array_mut())
    else {
        return Some((text.to_string(), false));
    };
    let before = bundles.len();
    bundles.retain(|v| v.as_str() != Some(pkg));
    if bundles.len() == before {
        return Some((text.to_string(), false));
    }
    Some((serde_json::to_string_pretty(&json).ok()?, true))
}

/// 从 cordis.patch.yml 文本中摘除 `name` 等于 pkg 的激活条目。
/// 返回 `Some((新文本, 是否有改动))`；顶层不是 patch 数组时返回 None
/// （认不出就不动文件 —— 写坏 patch 层比留着重复条目严重得多）。
fn strip_from_patch_yaml(text: &str, pkg: &str) -> Option<(String, bool)> {
    let mut entries: serde_yaml::Value = serde_yaml::from_str(text).ok()?;
    let Some(list) = entries.as_sequence_mut() else {
        return None;
    };
    let before = list.len();
    list.retain(|item| item.get("name").and_then(|v| v.as_str()) != Some(pkg));
    if list.len() == before {
        return Some((text.to_string(), false));
    }
    Some((serde_yaml::to_string(&entries).ok()?, true))
}

// ── 落盘 ────────────────────────────────────────────────────────

fn profile_dir() -> std::path::PathBuf {
    get_dsh_home().join("profiles").join(plugins_bundle::PROFILE)
}

/// 把 pkg 的独立挂载从 profile 中摘除。返回是否有任何改动。
/// 只动挂载凭据（bundles / patch），对「没有独立挂载」的包是无害空操作。
fn remove_standalone_mount(pkg: &str) -> Result<bool, String> {
    // 安全阀：聚合包本身是受控安装的那份，任何时候都不能被摘
    if pkg == BUNDLE {
        log::warn!("[repair] {pkg} 是聚合包本身，拒绝摘除");
        return Ok(false);
    }

    let mut changed = false;

    let pkg_json = profile_dir().join("package.json");
    if let Ok(text) = std::fs::read_to_string(&pkg_json) {
        if let Some((new_text, did)) = strip_from_bundles(&text, pkg) {
            if did {
                std::fs::write(&pkg_json, &new_text)
                    .map_err(|e| format!("写 {} 失败: {e}", pkg_json.display()))?;
                log::info!("[repair] 已从 dsh.profile.bundles 摘除 {pkg}");
                changed = true;
            }
        }
    }

    let patch = profile_dir().join("cordis.patch.yml");
    if let Ok(text) = std::fs::read_to_string(&patch) {
        match strip_from_patch_yaml(&text, pkg) {
            Some((new_text, true)) => {
                std::fs::write(&patch, &new_text)
                    .map_err(|e| format!("写 {} 失败: {e}", patch.display()))?;
                log::info!("[repair] 已从 cordis.patch.yml 摘除 {pkg} 的激活条目");
                changed = true;
            }
            Some((_, false)) => {}
            None => log::warn!(
                "[repair] {} 不是可识别的 patch 数组，跳过（不动文件）",
                patch.display()
            ),
        }
    }

    Ok(changed)
}

/// 聚合包的子包清单：以聚合包自己声明的 dependencies 为权威，
/// 读不到时退回我们硬编码的核心子包列表。
fn bundle_subpackages() -> Vec<String> {
    let mut names: Vec<String> = plugins_bundle::REQUIRED_PACKAGES
        .iter()
        .map(|s| s.to_string())
        .collect();

    let modules = plugins_bundle::node_modules_dir();
    if let Some(dir) = plugins_bundle::resolve_package(&modules, BUNDLE) {
        if let Ok(text) = std::fs::read_to_string(dir.join("package.json")) {
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&text) {
                if let Some(deps) = json.get("dependencies").and_then(|d| d.as_object()) {
                    for k in deps.keys() {
                        if !names.iter().any(|n| n == k) {
                            names.push(k.clone());
                        }
                    }
                }
            }
        }
    }
    names
}

/// 预防性清理：聚合包挂载健在时，摘除其子包的存量独立挂载。
/// 返回被清理的包名列表。单个包清理失败不阻断其余包。
pub fn preflight_cleanup() -> Result<Vec<String>, String> {
    // 聚合包没挂载就没有「重复」可言 —— 此时独立安装的插件是用户唯一的
    // 界面来源，动了反而拆人家的功能。
    if !plugins_bundle::is_mounted() {
        return Ok(vec![]);
    }

    let mut removed = Vec::new();
    for pkg in bundle_subpackages() {
        match remove_standalone_mount(&pkg) {
            Ok(true) => removed.push(pkg),
            Ok(false) => {}
            Err(e) => log::warn!("[repair] 清理 {pkg} 的独立挂载失败：{e}"),
        }
    }
    if !removed.is_empty() {
        log::info!(
            "[repair] 预检清理了与聚合包重复挂载的插件：{}",
            removed.join("、")
        );
    }
    Ok(removed)
}

/// 自愈入口：dsh 启动报 duplicate prefix route 时调用。
/// 返回 `Ok(Some(pkg))` = 已摘除 pkg 的独立挂载，可以重试；
/// `Ok(None)` = 没认出来或没动手（走正常重试/报错路径）。
pub fn repair_duplicate_route_failure(err: &str) -> Result<Option<String>, String> {
    let Some(pkg) = extract_conflicting_package(err) else {
        return Ok(None);
    };
    // 错误本身就是「两份挂载并存」的证据，直接摘独立的那份
    if remove_standalone_mount(&pkg)? {
        log::info!("[repair] 检测到 {pkg} 重复挂载导致启动失败，已清理，准备重试");
        Ok(Some(pkg))
    } else {
        log::warn!("[repair] {pkg} 撞路由但在挂载凭据里没找到独立副本，放弃自动修复");
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── extract_conflicting_package ────────────────────────────

    // 测试项：从真实日志格式的单行错误中提取冲突包名
    // 测试目的：确保自愈能认出真机出现的原始错误形态
    #[test]
    fn extracts_pkg_from_real_log_line() {
        let line = "Error: dsh: plugin tree failed to load: failed to apply loader entry \
                    web-ui-better-sidebar (dsh-better-sidebar): webserver: duplicate prefix \
                    route \"/sidebar/api\"";
        assert_eq!(
            extract_conflicting_package(line).as_deref(),
            Some("dsh-better-sidebar")
        );
    }

    // 测试项：从带 stderr 尾部的多行拼接错误中提取包名
    // 测试目的：start_dsh 的重试循环拿到的是「错误 + 最近 30 行 stderr」，
    // 必须能在其中定位到目标行，不被无关堆栈行干扰
    #[test]
    fn extracts_pkg_from_error_with_stderr_tail() {
        let err = "dsh process exited before URL was printed\n\
                   --- dsh stderr (最近 30 行) ---\n\
                       at Fiber.execute (cordis/lib/index.js:1067:24)\n\
                     [cause]: Error: failed to apply loader entry web-ui-better-sidebar \
                   (dsh-better-sidebar): webserver: duplicate prefix route \"/sidebar/api\"\n\
                         at Proxy.register (dsh-host-webserver/lib/index.js:55:36)";
        assert_eq!(
            extract_conflicting_package(err).as_deref(),
            Some("dsh-better-sidebar")
        );
    }

    // 测试项：无关启动错误不误报
    // 测试目的：普通崩溃（缺模块等）不应触发摘挂载的自愈动作
    #[test]
    fn ignores_unrelated_startup_errors() {
        assert_eq!(
            extract_conflicting_package("Error: Cannot find module 'x', ERR_MODULE_NOT_FOUND"),
            None
        );
    }

    // 测试项：只有路由冲突字样但没有 loader entry 行时不猜包名
    // 测试目的：信息不足以定位冲突方时宁可放弃修复，也不能乱摘挂载
    #[test]
    fn skips_when_loader_entry_missing() {
        assert_eq!(
            extract_conflicting_package("webserver: duplicate prefix route \"/api/x\""),
            None
        );
    }

    // ── strip_from_bundles ─────────────────────────────────────

    const PKG_JSON_WITH_DUP: &str = r#"{
  "name": "dsh-profile-web",
  "private": true,
  "dependencies": {
    "@linxin666/dsh-web-ui-all": "0.2.1",
    "dsh-better-sidebar": "1.2.0"
  },
  "dsh": {
    "profile": {
      "bundles": [
        "@deepseek-ai/dsh-base",
        "@deepseek-ai/dsh-web-app",
        "@linxin666/dsh-web-ui-all",
        "dsh-better-sidebar"
      ]
    }
  }
}"#;

    // 测试项：bundles 摘除独立包并保留聚合包与其余字段
    // 测试目的：验证只删重复的那一条，官方 bundle、dependencies 原样保留
    #[test]
    fn strips_dup_from_bundles_keeps_rest() {
        let (out, changed) = strip_from_bundles(PKG_JSON_WITH_DUP, "dsh-better-sidebar").unwrap();
        assert!(changed);
        let json: serde_json::Value = serde_json::from_str(&out).unwrap();
        let bundles = json.pointer("/dsh/profile/bundles").unwrap().as_array().unwrap();
        assert_eq!(bundles.len(), 3);
        assert!(!bundles.iter().any(|v| v.as_str() == Some("dsh-better-sidebar")));
        assert!(json.pointer("/dsh/profile/bundles/0").is_some());
        // dependencies 是 pnpm 的安装记录，刻意不动
        assert!(json.pointer("/dependencies/dsh-better-sidebar").is_some());
        assert_eq!(json["name"], "dsh-profile-web");
    }

    // 测试项：bundles 里没有目标包时原文返回且不算改动
    // 测试目的：保证幂等 —— 干净 profile 上预检不应产生任何写入
    #[test]
    fn bundles_without_pkg_is_noop() {
        let (out, changed) = strip_from_bundles(PKG_JSON_WITH_DUP, "some-other-plugin").unwrap();
        assert!(!changed);
        assert_eq!(out, PKG_JSON_WITH_DUP);
    }

    // 测试项：没有 dsh.profile.bundles 键时不算改动
    // 测试目的：dsh 未写挂载凭据的 profile 不应被误改
    #[test]
    fn missing_bundles_key_is_noop() {
        let text = r#"{"name": "dsh-profile-web", "dependencies": {}}"#;
        let (out, changed) = strip_from_bundles(text, "dsh-better-sidebar").unwrap();
        assert!(!changed);
        assert_eq!(out, text);
    }

    // 测试项：非法 JSON 拒绝处理
    // 测试目的：解析失败时绝不能把半截内容写回文件
    #[test]
    fn invalid_json_returns_none() {
        assert_eq!(strip_from_bundles("{not json", "dsh-better-sidebar"), None);
    }

    // ── strip_from_patch_yaml ──────────────────────────────────

    // 测试项：patch 摘除同名激活条目并保留其余条目（含 config）
    // 测试目的：验证只删重复条目，用户的其它插件激活配置不受影响
    #[test]
    fn strips_matching_entry_keeps_others() {
        let text = "- id: my-timer\n  name: '@deepseek-ai/cordis-plugin-timer'\n  config:\n    interval: 60\n- id: legacy-sidebar\n  name: dsh-better-sidebar\n- id: another\n  name: '@some-org/other'\n";
        let (out, changed) = strip_from_patch_yaml(text, "dsh-better-sidebar").unwrap();
        assert!(changed);
        let list: Vec<serde_yaml::Value> = serde_yaml::from_str(&out).unwrap();
        assert_eq!(list.len(), 2);
        assert_eq!(list[0]["name"].as_str(), Some("@deepseek-ai/cordis-plugin-timer"));
        assert_eq!(list[0]["config"]["interval"].as_i64(), Some(60));
        assert_eq!(list[1]["name"].as_str(), Some("@some-org/other"));
    }

    // 测试项：无匹配条目时原文返回
    // 测试目的：保证幂等，无重复时不动用户的 patch 层
    #[test]
    fn patch_without_match_is_noop() {
        let text = "- id: a\n  name: '@x/y'\n";
        let (out, changed) = strip_from_patch_yaml(text, "dsh-better-sidebar").unwrap();
        assert!(!changed);
        assert_eq!(out, text);
    }

    // 测试项：顶层不是数组时拒绝处理
    // 测试目的：patch 层结构不符合预期时宁可放弃修复也不能写坏它
    #[test]
    fn non_array_patch_returns_none() {
        let text = "plugins:\n  - name: dsh-better-sidebar\n";
        assert_eq!(strip_from_patch_yaml(text, "dsh-better-sidebar"), None);
    }

    // 测试项：非法 YAML 拒绝处理
    // 测试目的：解析失败时绝不能把内容写回文件
    #[test]
    fn invalid_yaml_returns_none() {
        assert_eq!(
            strip_from_patch_yaml("key: [1, 2", "dsh-better-sidebar"),
            None
        );
    }
}
