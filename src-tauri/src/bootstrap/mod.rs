//! 引导编排：从「什么都没有」到「dsh web 就绪、插件装好」。
//!
//! 每个阶段都会向前端推进度事件。引导失败时用户面对的是一个空窗口，
//! 事件流是他唯一能看到的东西，所以宁可多报也不要静默。

pub mod dsh;
pub mod download;
pub mod mirror;
pub mod node;

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};

use crate::state::AppState;

pub const EVENT_PROGRESS: &str = "bootstrap:progress";
pub const EVENT_READY: &str = "bootstrap:ready";
pub const EVENT_FAILED: &str = "bootstrap:failed";
/// 非致命问题：引导会继续走完，但用户应该知道少了什么。
pub const EVENT_WARNING: &str = "bootstrap:warning";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Stage {
    CheckingNode,
    DownloadingNode,
    CheckingDsh,
    InstallingDsh,
    InitProfile,
    InstallingPlugins,
    VerifyingPlugins,
    StartingDsh,
    WaitingReady,
}

impl Stage {
    pub const TOTAL: u8 = 9;

    pub fn label(self) -> &'static str {
        match self {
            Self::CheckingNode => "检查 Node.js 环境",
            Self::DownloadingNode => "下载 Node.js 运行时",
            Self::CheckingDsh => "检查 dsh 版本",
            Self::InstallingDsh => "安装 DeepSeek Harness",
            Self::InitProfile => "初始化配置",
            Self::InstallingPlugins => "安装界面插件",
            Self::VerifyingPlugins => "校验插件挂载",
            Self::StartingDsh => "启动 dsh 服务",
            Self::WaitingReady => "等待服务就绪",
        }
    }

    pub fn index(self) -> u8 {
        match self {
            Self::CheckingNode => 1,
            Self::DownloadingNode => 2,
            Self::CheckingDsh => 3,
            Self::InstallingDsh => 4,
            Self::InitProfile => 5,
            Self::InstallingPlugins => 6,
            Self::VerifyingPlugins => 7,
            Self::StartingDsh => 8,
            Self::WaitingReady => 9,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Progress {
    pub stage: Stage,
    pub label: String,
    pub detail: Option<String>,
    /// 仅下载/安装类阶段有值，0.0 ~ 1.0
    pub fraction: Option<f64>,
    pub index: u8,
    pub total: u8,
    /// 瞬态进度（下载字节数、安装计数）：界面上只刷新「当前活动」一行，
    /// 不进「已完成」列表 —— 否则几十条进度刷屏会把真正的里程碑挤掉。
    pub transient: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReadyPayload {
    pub url: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WarningPayload {
    pub message: String,
}

/// 进度上报器。所有阶段共用，保证事件格式一致。
#[derive(Clone)]
pub struct Reporter {
    app: AppHandle,
}

impl Reporter {
    pub fn new(app: AppHandle) -> Self {
        Self { app }
    }

    fn emit(&self, p: Progress) {
        // 上报失败不应影响引导本身继续跑
        let _ = self.app.emit(EVENT_PROGRESS, p);
    }

    pub fn stage(&self, stage: Stage) {
        self.emit(Progress {
            stage,
            label: stage.label().to_string(),
            detail: None,
            fraction: None,
            index: stage.index(),
            total: Stage::TOTAL,
            transient: false,
        });
    }

    pub fn detail(&self, stage: Stage, detail: impl Into<String>) {
        self.emit(Progress {
            stage,
            label: stage.label().to_string(),
            detail: Some(detail.into()),
            fraction: None,
            index: stage.index(),
            total: Stage::TOTAL,
            transient: false,
        });
    }

    /// 瞬态活动：马上会被下一条覆盖的那种进度（安装计数、校验中）。
    pub fn activity(&self, stage: Stage, detail: impl Into<String>, fraction: Option<f64>) {
        self.emit(Progress {
            stage,
            label: stage.label().to_string(),
            detail: Some(detail.into()),
            fraction,
            index: stage.index(),
            total: Stage::TOTAL,
            transient: true,
        });
    }

    /// 下载进度。total 未知时（服务端没给 Content-Length）只报已下载量。
    pub fn download(&self, stage: Stage, done: u64, total: Option<u64>) {
        let (detail, fraction) = match total {
            Some(t) if t > 0 => (
                format!("{:.1} / {:.1} MB", mb(done), mb(t)),
                Some(done as f64 / t as f64),
            ),
            _ => (format!("{:.1} MB", mb(done)), None),
        };
        self.emit(Progress {
            stage,
            label: stage.label().to_string(),
            detail: Some(detail),
            fraction,
            index: stage.index(),
            total: Stage::TOTAL,
            transient: true,
        });
    }

    pub fn ready(&self, url: impl Into<String>) {
        let _ = self.app.emit(EVENT_READY, ReadyPayload { url: url.into() });
    }

    /// 非致命问题。引导继续走完，但要留下用户看得见的痕迹。
    pub fn warn(&self, message: impl Into<String>) {
        let message = message.into();
        log::warn!("[bootstrap] 警告：{message}");
        let _ = self.app.emit(EVENT_WARNING, WarningPayload { message });
    }

    pub fn fail(&self, err: &str) {
        log::error!("[bootstrap] 失败: {err}");
        let _ = self.app.emit(EVENT_FAILED, err);
    }
}

fn mb(bytes: u64) -> f64 {
    bytes as f64 / 1024.0 / 1024.0
}

/// 引导主流程（Tauri 命令入口）。失败时把错误推给前端，由用户决定是否重试。
#[tauri::command]
pub async fn start_bootstrap(app: AppHandle) -> Result<(), String> {
    let reporter = Reporter::new(app.clone());

    match pipeline(&app, &reporter).await {
        Ok(url) => {
            // 把最终 URL 写进 ProcessState（复用现有状态流）
            let state = app.state::<AppState>();
            let pid = state.get_dsh_pid();
            let port = url
                .rsplit(':')
                .next()
                .and_then(|s| s.parse::<u16>().ok());
            let running = crate::state::ProcessState {
                status: "Running".to_string(),
                url: Some(url.clone()),
                port,
                pid: Some(pid),
                error: None,
                started_at: Some(crate::process::chrono_now()),
            };
            {
                let mut s = state.process_state.lock().await;
                *s = running.clone();
            }
            let _ = app.emit("process-state-changed", &running);
            reporter.ready(url);
            Ok(())
        }
        Err(e) => {
            reporter.fail(&e);
            Err(e)
        }
    }
}

async fn pipeline(app: &AppHandle, reporter: &Reporter) -> Result<String, String> {
    let _node = ensure_node(reporter).await?;
    let _entry = ensure_dsh(reporter).await?;

    // dsh 首次运行会自行创建 ~/.dsh 与 web profile，不需要显式 init
    reporter.stage(Stage::InitProfile);

    // 界面插件（鲸鱼娘 / 任务看板 / 皮肤中心 / 右侧面板都在聚合包里）。
    // 装不上不阻断启动 —— DSH 本体不依赖它们，装失败顶多少了界面功能。
    if let Err(e) = ensure_plugins(app, reporter).await {
        reporter.warn(format!(
            "界面插件未安装成功（{e}）。DSH 可正常使用，但鲸鱼娘、任务看板等界面功能不会出现，可稍后在插件市场重装。"
        ));
    }

    // ---- 启动 dsh（复用现有 start_dsh：3 次重试 + Job Object + stderr 尾部）----
    reporter.stage(Stage::StartingDsh);
    reporter.activity(
        Stage::StartingDsh,
        "正在启动服务；首次启动可能需要一两分钟",
        None,
    );

    let state = app.state::<AppState>();
    let running = crate::process::start_dsh(state, app.clone()).await?;

    let url = running.url.ok_or("dsh 已启动但未提供地址")?;

    reporter.stage(Stage::WaitingReady);
    reporter.detail(Stage::WaitingReady, "服务已启动，即将进入…");

    Ok(url)
}

/// 三级降级：系统 Node → 已装的便携版 → 下载便携版
async fn ensure_node(reporter: &Reporter) -> Result<node::NodeInfo, String> {
    reporter.stage(Stage::CheckingNode);

    if let Some(info) = node::detect_system_node() {
        reporter.detail(Stage::CheckingNode, format!("使用系统 Node.js {}", info.version));
        return Ok(info);
    }

    if let Some(info) = download::installed_portable_node() {
        reporter.detail(Stage::CheckingNode, format!("使用便携版 Node.js {}", info.version));
        return Ok(info);
    }

    // 明确区分「没装」与「装了但版本不对」—— 用户的后续动作完全不同
    let why = match node::system_node_version() {
        Some(v) => format!("系统 Node.js {v} 不满足要求，将下载便携版"),
        None => "未检测到 Node.js，将下载便携版".to_string(),
    };
    reporter.detail(Stage::CheckingNode, why);

    reporter.stage(Stage::DownloadingNode);
    let info = download::install_portable_node(reporter).await?;
    reporter.detail(Stage::DownloadingNode, format!("便携版 Node.js {} 就绪", info.version));
    Ok(info)
}

/// 定位 dsh，没有就装。
async fn ensure_dsh(reporter: &Reporter) -> Result<std::path::PathBuf, String> {
    reporter.stage(Stage::CheckingDsh);

    match dsh::entry_point()? {
        Some(entry) => {
            let label = match dsh::installed_version(&entry) {
                Some(v) => format!("已安装 dsh {v}"),
                None => "已安装 dsh".to_string(),
            };
            reporter.detail(Stage::CheckingDsh, label);
            Ok(entry)
        }
        None => {
            reporter.stage(Stage::InstallingDsh);
            // npm 安装是阻塞的且可能跑 1-3 分钟，扔到阻塞线程池，
            // 否则会占死一个 tokio worker，进度事件都推不出去。
            let rep = reporter.clone();
            tokio::task::spawn_blocking(move || dsh::install(&rep))
                .await
                .map_err(|e| format!("安装任务异常退出：{e}"))?
        }
    }
}

/// 安装界面插件并自检。
/// 装不上的表现是「DSH 能用但侧边栏空空、鲸鱼娘不出现」。
async fn ensure_plugins(app: &AppHandle, reporter: &Reporter) -> Result<(), String> {
    reporter.stage(Stage::InstallingPlugins);

    // 复用 M2 的全家桶：先检查，未安装/需修复才装
    let status = crate::plugins_bundle::check_bundle_status().await?;
    if status.status == "installed" {
        reporter.detail(Stage::InstallingPlugins, "界面插件已就绪");
    } else {
        // 干净机器上 dsh 装插件时内部调 pnpm，先补齐（同步阻塞，扔线程池）
        if !dsh::pnpm_available() {
            reporter.detail(Stage::InstallingPlugins, "正在安装 pnpm");
            tokio::task::spawn_blocking(dsh::install_pnpm)
                .await
                .map_err(|e| format!("安装 pnpm 任务异常退出：{e}"))??;
        }
        reporter.detail(
            Stage::InstallingPlugins,
            status
                .warning
                .unwrap_or_else(|| "正在安装界面插件全家桶（首次需要几分钟）".into()),
        );
        crate::plugins_bundle::install_bundle(app.clone()).await?;
    }

    reporter.stage(Stage::VerifyingPlugins);

    // 聚合包就位后先摘重复挂载：0.2.x 起聚合包内置 better-sidebar，从旧版
    // 升上来的 profile 里单独装过的会跟它撞 `/sidebar/api` 路由，dsh 直接
    // 起不来。此刻清掉，下面的首次启动就是干净的。
    match crate::profile_repair::preflight_cleanup() {
        Ok(removed) if !removed.is_empty() => reporter.warn(format!(
            "检测到与界面插件全家桶重复挂载的插件（{}），已自动清理。",
            removed.join("、")
        )),
        Ok(_) => {}
        Err(e) => log::warn!("[bootstrap] 插件挂载预检失败（不阻断启动）：{e}"),
    }

    match crate::plugins_bundle::verify_impl_public().await {
        Ok(Some(warning)) => reporter.warn(warning),
        Ok(None) => reporter.detail(Stage::VerifyingPlugins, "插件校验通过"),
        Err(e) => reporter.warn(e),
    }
    Ok(())
}
