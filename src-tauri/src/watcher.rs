//! 任务活动监视：轮询 dsh 的宠物状态接口，任务跑完时提醒用户。
//!
//! dsh-pet 插件在 dsh 服务端注册了同源 JSON 路由 `GET /api/pet/state`，
//! 返回当前会话的活动相位。`/api` 的 browser-trust 围栏只校验 Host 头
//! （防 DNS rebinding），loopback 客户端天然通过，无需注入页面。

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use tauri::{AppHandle, Manager};
use tauri_plugin_notification::NotificationExt;

use crate::state::AppState;

/// 轮询间隔（s）。
const POLL: Duration = Duration::from_secs(1);

/// 连续多少次连不上才放弃。按 1 秒间隔算，30 次约 30 秒。
const MAX_FAILURES: u8 = 30;

/// Windows toast 的提示音。notify-rust 对解析失败是 `.ok()` 静默吞掉，
/// 写错了就是没声音，不会报错。
const SOUND: &str = "Default";

/// 一次轮询的结果。
///
/// 「拿不到相位」和「连不上服务」必须分开：宠物在某些状态下响应里没有
/// `phase` 字段时，把两者都当失败计数，监视器会在几秒内耗尽失败次数
/// 永久退出 —— 表现就是「只有第一次任务有提醒」。
enum Poll {
    Phase {
        phase: String,
        turns: Option<u64>,
    },
    /// 服务正常，但这次响应里没有相位。不算故障，跳过即可。
    NoPhase,
    Unreachable,
}

/// 通知开关，供前端实时切换（不用重启监视器）。
#[derive(Clone, Default)]
pub struct NotifyEnabled(Arc<AtomicBool>);

impl NotifyEnabled {
    pub fn get(&self) -> bool {
        self.0.load(Ordering::Relaxed)
    }

    pub fn set(&self, value: bool) {
        self.0.store(value, Ordering::Relaxed);
    }
}

/// 在 JSON 里递归找第一个字符串型 `phase` 字段。
/// 上游没有对响应外层结构做稳定承诺，按字段名找，外层怎么变都不影响。
fn find_phase(value: &serde_json::Value) -> Option<String> {
    match value {
        serde_json::Value::Object(map) => {
            if let Some(serde_json::Value::String(p)) = map.get("phase") {
                return Some(p.clone());
            }
            map.values().find_map(find_phase)
        }
        serde_json::Value::Array(items) => items.iter().find_map(find_phase),
        _ => None,
    }
}

/// 在 JSON 里递归找第一个整数型 `turns` 字段。与 phase 一样不押注层级。
fn find_turns(value: &serde_json::Value) -> Option<u64> {
    match value {
        serde_json::Value::Object(map) => {
            if let Some(n) = map.get("turns").and_then(serde_json::Value::as_u64) {
                return Some(n);
            }
            map.values().find_map(find_turns)
        }
        serde_json::Value::Array(items) => items.iter().find_map(find_turns),
        _ => None,
    }
}

/// 启动后台监视。base_url 形如 `http://127.0.0.1:12345`。
pub fn spawn(app: AppHandle, base_url: String, enabled: NotifyEnabled) {
    tauri::async_runtime::spawn(async move {
        let Ok(client) = reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
        else {
            return;
        };

        let endpoint = format!("{}/api/pet/state", base_url.trim_end_matches('/'));
        let mut last_phase: Option<String> = None;
        let mut last_turns: Option<u64> = None;
        // 已经为哪个 turns 值提醒过。防止两个完成信号在同一拍重复触发。
        let mut notified_turns: Option<u64> = None;
        let mut failures: u8 = 0;

        loop {
            tokio::time::sleep(POLL).await;

            // 服务已换代（重启后换了端口），旧监视器退出。
            let current_url = app.state::<AppState>().process_state.lock().await.url.clone();
            if current_url.as_deref() != Some(base_url.as_str()) {
                log::info!("[watcher] 服务已换代，旧监视器退出（{base_url}）");
                return;
            }

            let (phase, turns) = match fetch(&client, &endpoint).await {
                Poll::Phase { phase, turns } => {
                    failures = 0;
                    (phase, turns)
                }
                // 服务活着，只是这一拍没有相位。保持 last_phase 不动——
                // 清掉的话下一次真正的 done 会因为「前一个相位未知」而漏报。
                Poll::NoPhase => {
                    failures = 0;
                    continue;
                }
                Poll::Unreachable => {
                    failures += 1;
                    if failures >= MAX_FAILURES {
                        log::warn!(
                            "[watcher] {endpoint} 连续 {MAX_FAILURES} 次连不上，停止监视。\
                             多半是宠物插件未安装，或 dsh 服务已异常。"
                        );
                        return;
                    }
                    continue;
                }
            };

            let prev_phase = last_phase.replace(phase.clone());
            let prev_turns = std::mem::replace(&mut last_turns, turns);

            // 完成信号有两个来源，任一命中即算完成：
            // 1. 相位切进 done（依赖恰好采到 done 那一拍，短回复会漏）；
            // 2. turns 增加（累积量，漏采多少拍都能补回来，根治性的）。
            let phase_done =
                phase == "done" && prev_phase.is_some() && prev_phase.as_deref() != Some("done");

            let turns_bumped =
                matches!((turns, prev_turns), (Some(now), Some(before)) if now > before);

            if !(phase_done || turns_bumped) {
                continue;
            }

            // 两个信号通常在**同一拍**同时命中，不去重会一次完成响两声。
            if turns.is_some() && turns == notified_turns {
                continue;
            }
            notified_turns = turns;

            if !enabled.get() {
                log::info!("[watcher] 任务完成，但通知已被关闭");
                continue;
            }
            if user_is_watching(&app) {
                log::info!("[watcher] 任务完成，主窗口在前台，不打扰");
                continue;
            }

            log::info!(
                "[watcher] 任务完成，发出提醒（触发源：{}）",
                if turns_bumped { "turns" } else { "相位" }
            );
            notify(&app);
            flash_main(&app);
            crate::tray::start_blink(&app);
        }
    });
}

async fn fetch(client: &reqwest::Client, endpoint: &str) -> Poll {
    let Ok(resp) = client.get(endpoint).send().await else {
        return Poll::Unreachable;
    };
    if !resp.status().is_success() {
        return Poll::Unreachable;
    }
    let Ok(json) = resp.json::<serde_json::Value>().await else {
        return Poll::Unreachable;
    };
    match find_phase(&json) {
        Some(phase) => Poll::Phase {
            phase,
            turns: find_turns(&json),
        },
        None => Poll::NoPhase,
    }
}

/// 用户此刻是不是真的在看主窗口。
/// 三个条件都满足才算「在看」：可见、没最小化、有焦点。
fn user_is_watching(app: &AppHandle) -> bool {
    let Some(w) = app.get_webview_window("main") else {
        return false;
    };
    w.is_visible().unwrap_or(false)
        && !w.is_minimized().unwrap_or(false)
        && w.is_focused().unwrap_or(false)
}

/// 发系统通知。
fn notify(app: &AppHandle) {
    let result = app
        .notification()
        .builder()
        .title("DSH Agent")
        .body("任务已完成")
        .sound(SOUND)
        .show();
    if let Err(e) = result {
        log::warn!("[watcher] 构造通知失败：{e}");
    }
}

/// 任务栏图标闪烁（FLASHW_TIMERNOFG：一直闪到窗口进入前台为止）。
#[cfg(target_os = "windows")]
fn flash_main(app: &AppHandle) {
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        FlashWindowEx, FLASHWINFO, FLASHW_ALL, FLASHW_TIMERNOFG,
    };

    let Some(w) = app.get_webview_window("main") else {
        return;
    };
    let Ok(handle) = w.hwnd() else {
        return;
    };
    // tauri 依赖的 windows crate 与我们直接依赖的 windows-sys 是两个版本，
    // HWND 类型不同，取原始指针（*mut c_void）即可——windows-sys 的 HWND
    // 就是 *mut c_void 的类型别名。
    let info = FLASHWINFO {
        cbSize: std::mem::size_of::<FLASHWINFO>() as u32,
        hwnd: handle.0,
        dwFlags: FLASHW_ALL | FLASHW_TIMERNOFG,
        uCount: 0,
        dwTimeout: 0,
    };
    unsafe {
        let _ = FlashWindowEx(&info);
    }
}

#[cfg(not(target_os = "windows"))]
fn flash_main(_app: &AppHandle) {}
