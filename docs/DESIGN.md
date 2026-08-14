# DSH Agent — 技术设计文档

> DeepSeek Harness 桌面 Webview 包装器：一键启动 `dsh web` 并在原生 webview 中加载的跨平台桌面应用。
>
> 上游产品分析见 [DSH-ANALYSIS.md](./DSH-ANALYSIS.md)。

## 1. 项目概述

### 1.1 目标

将 DeepSeek Harness 的 Web UI（`dsh web`）包装为一个**一键启动的独立桌面应用**，用户无需打开终端、输入命令、再手动在浏览器中访问 URL——双击 App 即可使用。

### 1.2 核心价值

- **零终端操作**：用户双击图标，App 自动在后台启动 `dsh web` 服务并加载 UI
- **环境自检**：检测 Node.js 版本、dsh 安装状态并引导修复
- **版本更新**：自动检查 dsh 和 App 自身的版本更新
- **插件市场**：浏览 GitHub 上 `dsh-plugin` topic 的插件仓库，一键安装到当前 profile
- **跨平台**：Windows / macOS / Linux 统一代码库

### 1.3 技术栈选型

| 层 | 技术 | 版本 | 理由 |
|---|---|---|---|
| **桌面框架** | Tauri 2.x | ≥2.0 | 包体积极小（~3-5MB），跨平台系统 webview，Rust 后端安全高效 |
| **前端框架** | SvelteKit (静态 SPA) | ≥2.0 | 轻量、编译产物小，适合 Tauri 的 webview 场景 |
| **前端 UI** | Tailwind CSS + shadcn-svelte | latest | 快速构建美观的设置/环境检查界面 |
| **后端** | Rust (Tauri 内置) | edition 2021 | 进程管理、文件系统操作、版本检查、HTTP 请求 |
| **包管理** | pnpm + Cargo | latest | 前端用 pnpm，Rust 依赖用 Cargo |

### 1.4 为什么选 Tauri 而不是 Electron

| 维度 | Tauri | Electron |
|------|-------|----------|
| 安装包体积 | ~3-5 MB | ~80+ MB |
| 内存占用 | ~30-50 MB | ~150+ MB |
| 渲染引擎 | 系统 webview (Win: WebView2, Mac: WKWebView, Linux: WebKitGTK) | 内置 Chromium |
| 后端语言 | Rust（内存安全，无 GC 暂停） | Node.js |
| 进程管理 | Rust `tokio::process` + `Command` | `child_process` |
| 跨平台一致性 | 略有差异（系统 webview 不同） | 完全一致（同一 Chromium 版本） |

对于本项目而言，dsh web 前端已经是标准 SPA，对 webview 引擎差异不敏感；而 Tauri 的体积和安全优势是决定性的。

---

## 2. 系统架构

### 2.1 整体架构图

```
┌───────────────────────────────────────────────────────────┐
│                     DSH Agent App                          │
│                                                            │
│  ┌───────────────────┐    ┌─────────────────────────────┐│
│  │  Tauri Frontend    │    │      Tauri Backend (Rust)    ││
│  │  (SvelteKit SPA)   │    │                              ││
│  │                     │    │  ┌────────────────────────┐││
│  │  ┌───────────────┐ │    │  │ Process Manager         │││
│  │  │  Main View    │◄┼────┼──│  (spawn dsh web)        │││
│  │  │  (webview)    │ │    │  └─────────┬──────────────┘││
│  │  └───────────────┘ │    │            │                ││
│  │                     │    │  ┌────────▼──────────────┐││
│  │  ┌───────────────┐ │    │  │ stdout Parser           │││
│  │  │  Env Panel    │◄┼────┼──│  (port extract)        │││
│  │  └───────────────┘ │    │  └────────────────────────┘││
│  │  ┌───────────────┐ │    │  ┌────────────────────────┐││
│  │  │  Version Panel│◄┼────┼──│ Env Checker + Updater   │││
│  │  └───────────────┘ │    │  │  (node, dsh, npm)       │││
│  │  ┌───────────────┐ │    │  └────────────────────────┘││
│  │  │  Plugin Market│◄┼────┼──┌────────────────────────┐││
│  │  │  Panel        │ │    │  │ Plugin Market          │││
│  │  └───────────────┘ │    │  │  (GitHub API + dsh     │││
│  │                     │    │  │   plugin install)     │││
│  └─────────────────────┘    │  └────────────────────────┘││
│                              └─────────────────────────────┘│
└──────────────────────────────────────────────────────────────┘
                                     │ spawn
                                     ▼
                         ┌─────────────────────┐
                         │   dsh web 进程       │
                         │ (Node.js HTTP srv)   │
                         │ 127.0.0.1:<port>     │
                         └─────────────────────┘
```

### 2.2 模块职责

| 模块 | 位置 | 职责 |
|------|------|------|
| **ProcessManager** | `src-tauri/src/process.rs` | spawn/kill `dsh web`，管理进程生命周期 |
| **StdoutParser** | `src-tauri/src/process.rs` | 解析 stdout 中的 `dsh web: http://127.0.0.1:<port>` 行 |
| **EnvChecker** | `src-tauri/src/env.rs` | 检测 Node.js 版本、dsh 安装状态、npm 可用性 |
| **VersionChecker** | `src-tauri/src/version.rs` | 查询 npm registry 获取 dsh 最新版本，与本地对比 |
| **Installer** | `src-tauri/src/installer.rs` | 执行 `npm install -g @deepseek-ai/dsh` 并流式输出进度 |
| **PluginMarket** | `src-tauri/src/plugins.rs` | 调用 GitHub Search API 浏览 dsh-plugin 仓库，调用 `dsh plugin` 安装/移除 |
| **MainView** | `src/lib/views/Main.svelte` | webview 主窗口，加载 dsh web URL |
| **EnvPanel** | `src/lib/views/EnvPanel.svelte` | 运行环境检查面板 |
| **VersionPanel** | `src/lib/views/VersionPanel.svelte` | 版本更新检查面板 |
| **PluginMarketPanel** | `src/lib/views/PluginMarket.svelte` | 插件市场浏览与安装面板 |

---

## 3. 进程管理

### 3.1 dsh web 启动流程

```
App 启动
  │
  ▼
EnvChecker.run()
  │── 检查 Node.js: `node --version` → parse → ≥22 ?
  │── 检查 dsh:    `dsh --version`   → parse → 0.1.0-rc.6 ?
  │── 检查 npm:    `npm --version`   → 可用 ?
  │
  ├─ 缺失 → 跳转 EnvPanel 引导安装
  │
  └─ 全部就绪 → ProcessManager.start()
      │
      │ spawn: dsh web --port 0
      │
      ▼
    StdoutParser 逐行读取 stdout
      │
      │ 匹配正则: /^dsh web:\s+http:\/\/127\.0\.0\.1:(\d+)/
      │
      ├─ 匹配成功 → 提取 port → 返回 URL
      │                  │
      │                  ▼
      │            MainView 加载 http://127.0.0.1:<port>
      │
      └─ 超时 (30s) → 报错，显示启动失败面板
```

### 3.2 stdout 输出格式

`dsh web` 启动后会在 stdout 输出以下格式的 URL 行：

```
dsh web: http://127.0.0.1:3080
```

**解析正则**：

```rust
let re = regex::Regex::new(r"^dsh web:\s+http://127\.0\.0\.1:(\d+)")?;
```

### 3.3 端口分配策略

使用 `--port 0` 让操作系统自动分配空闲端口，避免：

- 默认端口 3080 被占用
- 多实例冲突
- 防火墙规则残留

### 3.4 进程终止策略

App 窗口关闭时：

1. 优先发送 `SIGTERM`（Unix）或 `taskkill /pid <pid> /T`（Windows）
2. 等待 5 秒
3. 如果进程仍存活，强制 `SIGKILL`（Unix）或 `taskkill /pid <pid> /F /T`（Windows）

Rust 实现：

```rust
pub async fn kill_dsh(&self) -> Result<()> {
    if let Some(child) = &self.child {
        #[cfg(target_os = "windows")]
        {
            let pid = child.id();
            Command::new("taskkill")
                .args(["/pid", &pid.to_string(), "/T", "/F"])
                .output()
                .await?;
        }
        #[cfg(not(target_os = "windows"))]
        {
            let pid = child.id();
            Command::new("kill")
                .args(["-TERM", &pid.to_string()])
                .output()
                .await?;
            tokio::time::sleep(Duration::from_secs(5)).await;
            Command::new("kill")
                .args(["-KILL", &pid.to_string()])
                .output()
                .await?;
        }
    }
    Ok(())
}
```

> **Windows 注意**：`dsh` 是一个 Node.js 脚本，`dsh web` 实际上会启动 `node` 进程。`taskkill /T` 会递归终止子进程树，确保 Node 进程也被清理。

### 3.5 进程状态管理

```rust
pub struct ProcessState {
    status: ProcessStatus,
    url: Option<String>,
    port: Option<u16>,
    pid: Option<u32>,
    error: Option<String>,
    started_at: Option<DateTime<Utc>>,
}

pub enum ProcessStatus {
    NotStarted,
    Starting,
    Running,
    Stopping,
    Stopped,
    Failed(String),
}
```

前端通过 Tauri command 获取状态，并订阅状态变更事件：

```typescript
import { listen } from '@tauri-apps/api/event';
await listen<ProcessState>('process-state-changed', (event) => {
    // 更新 UI
});
```

---

## 4. 运行环境检查

### 4.1 检查项

| 检查项 | 命令 | 期望 | 处理 |
|--------|------|------|------|
| **Node.js** | `node --version` | ≥ 22.0.0 | 未安装/版本过低 → 显示下载链接 |
| **npm** | `npm --version` | 可用 | 未安装 → 提示随 Node.js 一起安装 |
| **dsh** | `dsh --version` | ≥ 0.1.0-rc.6 | 未安装 → 一键安装按钮 |
| **DSH_HOME** | `~/.dsh` 存在 | 目录存在 | 不存在 → 首次运行会自动创建 |

### 4.2 版本解析

```rust
// Node.js 版本: "v24.14.1" → 24.14.1
fn parse_node_version(output: &str) -> Option<Version> {
    let v = output.trim().trim_start_matches('v');
    Version::parse(v).ok()
}

// dsh 版本: "0.1.0-rc.6" → 需自定义解析（含 prerelease tag）
fn parse_dsh_version(output: &str) -> Option<DshVersion> {
    semver::Version::parse(output.trim()).ok()
}
```

### 4.3 环境检查面板

```
┌─────────────────────────────────────────┐
│  运行环境                                │
├─────────────────────────────────────────┤
│                                         │
│  ✅ Node.js              v24.14.1      │
│     路径: D:\Files\DevEvn\nvmd\...      │
│     最低要求: v22.0.0                   │
│                                         │
│  ✅ npm                  11.11.0        │
│     路径: D:\Files\DevEvn\nvmd\...      │
│                                         │
│  ✅ dsh (DeepSeek Harness)              │
│     版本: 0.1.0-rc.6                    │
│     路径: C:\Users\makise\.nvmd\bin\dsh │
│     [更新到最新版]                       │
│                                         │
│  ✅ DSH_HOME             C:\Users\...    │
│     profiles/sessions/storages 就绪      │
│                                         │
│  ─────────────────────────────────────  │
│                                         │
│  [重新检查]    [一键安装/更新 dsh]       │
│                                         │
└─────────────────────────────────────────┘
```

### 4.4 PATH 解析

在 Windows 上，`dsh` 可能安装在不同位置（nvmd、nvm-windows、全局 npm prefix 等）。检测策略：

```rust
async fn find_dsh_path() -> Option<String> {
    let cmd = if cfg!(target_os = "windows") { "where" } else { "which" };
    let output = Command::new(cmd).arg("dsh").output().await.ok()?;
    let paths = String::from_utf8_lossy(&output.stdout);
    paths.lines().next().map(|s| s.to_string())
}
```

---

## 5. 版本更新检查

### 5.1 dsh 版本更新

通过 npm registry API 检查最新版本：

```
GET https://registry.npmjs.org/@deepseek-ai/dsh/latest
→ { "version": "0.1.0-rc.7", ... }
```

Rust 实现：

```rust
async fn check_dsh_update(current: &str) -> Result<Option<String>> {
    let resp: serde_json::Value = reqwest::Client::new()
        .get("https://registry.npmjs.org/@deepseek-ai/dsh/latest")
        .send().await?
        .json().await?;
    let latest = resp["version"].as_str().unwrap_or("");
    if latest != current && !latest.is_empty() {
        Ok(Some(latest.to_string()))
    } else {
        Ok(None)
    }
}
```

### 5.2 dsh 安装/更新流程

```
用户点击"安装/更新 dsh"
  │
  ▼
Rust 后端 spawn: npm install -g @deepseek-ai/dsh@latest
  │
  ├── stdout/stderr → 事件流 → 前端实时显示进度
  │
  └── 进程退出码
      ├─ 0: 成功 → 重新运行环境检查 → 刷新版本号
      └─ 非0: 失败 → 显示错误日志
```

### 5.3 App 自身更新

使用 Tauri 内置的 updater 插件（`tauri-plugin-updater`）：

```json
{
  "plugins": {
    "updater": {
      "active": true,
      "endpoints": [
        "https://github.com/your-name/dsh-agent/releases/latest/download/latest.json"
      ],
      "pubkey": "<ED25519_PUBLIC_KEY>"
    }
  }
}
```

### 5.4 版本面板 UI

```
┌─────────────────────────────────────────┐
│  版本与更新                             │
├─────────────────────────────────────────┤
│                                         │
│  DSH Agent (本应用)                     │
│  当前版本: 1.0.0                        │
│  [检查更新]                              │
│                                         │
│  ─────────────────────────────────────  │
│                                         │
│  dsh (DeepSeek Harness)                 │
│  当前版本: 0.1.0-rc.6                   │
│  最新版本: 0.1.0-rc.7    (有新版本)     │
│  [更新到 0.1.0-rc.7]                    │
│                                         │
│  ─────────────────────────────────────  │
│                                         │
│  更新日志                                │
│  [显示 dsh changelog]                   │
│                                         │
└─────────────────────────────────────────┘
```

---

## 6. 插件市场

### 6.1 数据源

通过 GitHub Search API 获取带有 `dsh-plugin` topic 的仓库：

```
GET https://api.github.com/search/repositories?q=topic:dsh-plugin&sort=stars&order=desc&per_page=30
```

返回标准 GitHub Repo JSON（当前约 834+ 个仓库），包含以下可用字段：

| 字段 | 用途 |
|------|------|
| `full_name` | 仓库名 (owner/repo) |
| `description` | 仓库描述 |
| `html_url` | 仓库链接 |
| `stargazers_count` | Star 数 (排序依据) |
| `topics` | 所有 topic 标签 |
| `updated_at` | 最后更新时间 |
| `owner.avatar_url` | 作者头像 |
| `license` | 许可证信息 |
| `open_issues_count` | Issue 数 |

### 6.2 已安装插件检测

通过 `dsh plugin --profile web list` 获取当前 profile 已安装的 pnpm 包列表，与 GitHub 市场数据交叉比对，标记已安装/未安装状态。

### 6.3 插件安装/移除

dsh 自带的插件管理通过 `dsh plugin` 命令（转发给 pnpm）：

```bash
# 安装
dsh plugin --profile web add <package-name>

# 移除
dsh plugin --profile web remove <package-name>
```

App 封装为 Tauri command，spawn 上述命令并流式输出 stdout/stderr 到前端。

### 6.4 插件激活

安装 npm 包后，插件还需要在 profile 的 `cordis.patch.yml` 中添加 patch 行才能激活。App 的安装流程包含：

1. `dsh plugin --profile web add <package>` (安装 npm 包)
2. 读取 `$DSH_HOME/profiles/web/cordis.patch.yml`
3. 追加 patch 条目（询问用户确认配置）
4. 写回 `cordis.patch.yml`
5. 提示用户重启 dsh 进程使插件生效

### 6.5 插件市场面板 UI

```
┌──────────────────────────────────────────────────────────────────┐
│  插件市场                                          [刷新]         │
├──────────────────────────────────────────────────────────────────┤
│  🔍 搜索插件...                                    排序: ⭐ Stars │
├──────────────────────────────────────────────────────────────────┤
│                                                                  │
│  ┌────────────────────────────────────────────────────────────┐ │
│  │ 📦 @deepseek-ai/awesome-dsh-plugin        ⭐ 1.2k   已安装  │ │
│  │    An awesome plugin for DeepSeek Harness                  │ │
│  │    更新于 2026-08-10    License: MIT                        │ │
│  │                                             [移除]          │ │
│  └────────────────────────────────────────────────────────────┘ │
│                                                                  │
│  ┌────────────────────────────────────────────────────────────┐ │
│  │ 📦 @user/some-dsh-tool                   ⭐ 340    安装    │ │
│  │    A tool plugin that does something cool                  │ │
│  │    更新于 2026-08-12    License: MIT                        │ │
│  │                                             [安装]          │ │
│  └────────────────────────────────────────────────────────────┘ │
│                                                                  │
│  ┌────────────────────────────────────────────────────────────┐ │
│  │ 📦 @dev/another-plugin                    ⭐ 89     安装    │ │
│  │    Another useful plugin                                   │ │
│  │    更新于 2026-08-08    License: Apache-2.0                │ │
│  │                                             [安装]          │ │
│  └────────────────────────────────────────────────────────────┘ │
│                                                                  │
│  [加载更多]                                                       │
│                                                                  │
└──────────────────────────────────────────────────────────────────┘
```

### 6.6 安装进度

```
正在安装 @user/some-dsh-tool...
┌──────────────────────────────────────────────────┐
│  Progress: ████████████████░░░░░░  68%           │
│                                                  │
│  Packages: +42                                   │
│  Progress: resolving... downloading...           │
│  ────────────────────────────────────────────    │
│  [查看详细日志]                  [取消]          │
└──────────────────────────────────────────────────┘
```

### 6.7 搜索与过滤

- **搜索框**：按仓库名/描述模糊搜索 (客户端过滤当前页，或 GitHub API `q` 参数)
- **排序**：Stars / 最近更新 / 字母序
- **过滤**：全部 / 已安装 / 未安装
- **分页**：GitHub API `page` 参数，每页 30 条

---

## 7. 前端架构

### 7.1 路由结构

```
/               → 主窗口 (webview 加载 dsh web)
/env            → 运行环境检查面板
/version        → 版本与更新面板
/plugins        → 插件市场面板
/settings      → 应用设置
```

### 7.2 主窗口行为

主窗口加载 `http://127.0.0.1:<port>`：

```svelte
<!-- Main.svelte -->
<script lang="ts">
    let loading = true;
    let url = '';
    let error = '';

    onMount(async () => {
        const state = await invoke('start_dsh');
        url = state.url;
        loading = false;
    });
</script>

{#if loading}
    <LoadingScreen />
{:else if error}
    <ErrorScreen {error} />
{:else}
    <iframe src={url} class="w-full h-full border-0" />
{/if}
```

> **注意**：Tauri 2.x 的主 webview 不能直接加载远程 URL 作为主页面（安全限制）。方案是在主窗口中用 `<iframe>` 加载 dsh web URL，或通过 Tauri 的 `WebviewWindow::new` 创建第二个窗口。另一个更简洁的方案是主窗口直接 `window.location.href = url`，在 Tauri 的 `WebviewUrl::External` 模式下导航。

### 7.3 顶部导航栏

顶部水平导航栏，包含应用名称、页面切换标签和状态指示：

```
┌──────────────────────────────────────────────────────────────────┐
│  DSH Agent  [ 主界面 ] [ 运行环境 ] [ 版本更新 ] [ 插件市场 ]  ● 运行中  │
├──────────────────────────────────────────────────────────────────┤
│                                                                  │
│                     主区域 (webview 或面板)                      │
│                                                                  │
│                                                                  │
└──────────────────────────────────────────────────────────────────┘
```

导航项：
- **主界面**：返回 dsh web 主界面 (webview)
- **运行环境**：运行环境检查面板
- **版本更新**：版本更新检查面板
- **插件市场**：插件浏览与安装面板

右侧状态指示器：
- ● 运行中 (绿色) / ● 启动中 (黄色脉冲) / ● 已停止 (灰色) / ● 错误 (红色)

顶部导航栏固定高度 (40px)，主区域填满剩余空间。

### 7.4 状态管理

使用 Svelte 的 store 管理全局状态：

```typescript
// stores/app.ts
import { writable } from 'svelte/store';

export const processState = writable<ProcessState>({
    status: 'NotStarted',
    url: null,
    port: null,
    error: null,
});

export const envState = writable<EnvState>({
    node: { installed: false, version: null, path: null },
    npm: { installed: false, version: null, path: null },
    dsh: { installed: false, version: null, path: null },
    dshHome: { exists: false, path: null },
});

export const pluginState = writable<PluginState>({
    marketRepos: [],
    installedPackages: [],
    loading: false,
    installingPackage: null,
});
```

---

## 8. Tauri Command API

### 8.1 进程管理 Commands

```rust
#[tauri::command]
async fn start_dsh(state: State<AppState>) -> Result<ProcessState, String>;

#[tauri::command]
async fn stop_dsh(state: State<AppState>) -> Result<(), String>;

#[tauri::command]
async fn get_dsh_status(state: State<AppState>) -> Result<ProcessState, String>;

#[tauri::command]
async fn restart_dsh(state: State<AppState>) -> Result<ProcessState, String>;
```

### 8.2 环境检查 Commands

```rust
#[tauri::command]
async fn check_environment() -> Result<EnvState, String>;

#[tauri::command]
async fn check_node_version() -> Result<NodeInfo, String>;

#[tauri::command]
async fn check_dsh_version() -> Result<DshInfo, String>;
```

### 8.3 安装/更新 Commands

```rust
#[tauri::command]
async fn install_dsh(app: AppHandle) -> Result<(), String>;
// 通过 app.emit("install-progress", payload) 流式输出进度

#[tauri::command]
async fn check_dsh_update() -> Result<UpdateInfo, String>;

#[tauri::command]
async fn check_app_update(app: AppHandle) -> Result<UpdateInfo, String>;
```

### 8.4 插件市场 Commands

```rust
#[tauri::command]
async fn search_plugins(
    query: Option<String>,
    sort: Option<String>,    // "stars" | "updated" | "name"
    page: Option<u32>,       // 默认 1
) -> Result<PluginSearchResult, String>;
// 调用 GitHub Search API

#[tauri::command]
async fn list_installed_plugins() -> Result<Vec<InstalledPlugin>, String>;
// 调用 dsh plugin --profile web list

#[tauri::command]
async fn install_plugin(app: AppHandle, package: String) -> Result<(), String>;
// 调用 dsh plugin --profile web add <package>
// 通过 app.emit("plugin-install-progress", payload) 流式输出

#[tauri::command]
async fn remove_plugin(app: AppHandle, package: String) -> Result<(), String>;
// 调用 dsh plugin --profile web remove <package>

#[tauri::command]
async fn activate_plugin(plugin_id: String, plugin_name: String) -> Result<(), String>;
// 编辑 cordis.patch.yml 追加 patch 条目
```

### 8.5 事件定义

| 事件名 | Payload | 触发时机 |
|--------|---------|----------|
| `process-state-changed` | `ProcessState` | dsh 进程状态变更 |
| `install-progress` | `{ stage, message, percent }` | npm install 进度 |
| `env-check-completed` | `EnvState` | 环境检查完成 |
| `update-available` | `UpdateInfo` | 发现新版本 |
| `plugin-install-progress` | `{ package, stage, message }` | 插件安装进度 |

---

## 9. 跨平台适配

### 9.1 平台差异矩阵

| 维度 | Windows | macOS | Linux |
|------|---------|-------|-------|
| WebView 引擎 | WebView2 (Edge/Chromium) | WKWebView (Safari/WebKit) | WebKitGTK |
| 进程终止 | `taskkill /pid <pid> /T /F` | `kill -TERM <pid>` | `kill -TERM <pid>` |
| 命令查找 | `where dsh` | `which dsh` | `which dsh` |
| Shell | PowerShell | zsh | bash |
| Node 安装器 | .msi / nvmd | .pkg / Homebrew | nvm / apt |
| DSH_HOME | `%USERPROFILE%\.dsh` | `~/.dsh` | `~/.dsh` |

### 9.2 条件编译

```rust
#[cfg(target_os = "windows")]
fn find_executable(name: &str) -> Command {
    Command::new("where").arg(name)
}

#[cfg(not(target_os = "windows"))]
fn find_executable(name: &str) -> Command {
    Command::new("which").arg(name)
}

#[cfg(target_os = "windows")]
fn kill_process_tree(pid: u32) -> Command {
    Command::new("taskkill").args(["/pid", &pid.to_string(), "/T", "/F"])
}

#[cfg(not(target_os = "windows"))]
fn kill_process_tree(pid: u32) -> Command {
    Command::new("kill").args(["-KILL", &pid.to_string()])
}
```

### 9.3 WebView2 运行时（Windows）

- Windows 11 自带 WebView2 Runtime
- Windows 10 可能需要安装（Tauri 安装包可内置 WebView2 bootstrapper）
- Tauri 配置：

```json
{
  "bundle": {
    "windows": {
      "webviewInstallMode": {
        "type": "downloadBootstrapper"
      }
    }
  }
}
```

---

## 10. 项目结构

```
dsh-agent/
├── docs/
│   ├── DESIGN.md              # 本文档（项目设计）
│   └── DSH-ANALYSIS.md        # 上游 dsh 分析文档
├── src-tauri/                  # Rust 后端
│   ├── Cargo.toml
│   ├── tauri.conf.json
│   ├── build.rs
│   ├── icons/
│   └── src/
│       ├── main.rs            # Tauri 入口，注册所有 commands
│       ├── process.rs         # dsh 进程管理
│       ├── env.rs             # 环境检查
│       ├── version.rs         # 版本检查
│       ├── installer.rs       # npm install 执行
│       ├── plugins.rs         # 插件市场（GitHub API + dsh plugin）
│       └── state.rs           # 全局状态定义
├── src/                       # 前端 (SvelteKit)
│   ├── app.html
│   ├── app.css
│   ├── lib/
│   │   ├── stores/
│   │   │   ├── app.ts         # 全局状态
│   │   │   ├── env.ts         # 环境状态
│   │   │   └── plugins.ts     # 插件市场状态
│   │   ├── views/
│   │   │   ├── Main.svelte     # 主窗口 (webview)
│   │   │   ├── EnvPanel.svelte # 运行环境面板
│   │   │   ├── VersionPanel.svelte # 版本更新面板
│   │   │   └── PluginMarket.svelte # 插件市场面板
│   │   ├── components/
│   │   │   ├── TopNav.svelte    # 顶部导航栏
│   │   │   ├── LoadingScreen.svelte
│   │   │   ├── StatusBadge.svelte
│   │   │   └── PluginCard.svelte # 插件卡片组件
│   │   └── api/
│   │       ├── tauri.ts       # Tauri command 封装
│   │       └── types.ts       # 共享类型定义
│   └── routes/
│       ├── +layout.svelte
│       ├── +page.svelte       # 主页 → Main
│       ├── env/+page.svelte   # /env → EnvPanel
│       ├── version/+page.svelte # /version → VersionPanel
│       └── plugins/+page.svelte # /plugins → PluginMarket
├── package.json
├── svelte.config.js
├── vite.config.ts
├── tailwind.config.js
└── tsconfig.json
```

---

## 11. 数据结构定义

### 11.1 Rust 侧（序列化给前端）

```rust
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Clone)]
pub struct ProcessState {
    pub status: String,        // "NotStarted" | "Starting" | "Running" | "Stopping" | "Stopped" | "Failed"
    pub url: Option<String>,   // "http://127.0.0.1:3080"
    pub port: Option<u16>,
    pub pid: Option<u32>,
    pub error: Option<String>,
    pub started_at: Option<String>, // ISO 8601
}

#[derive(Serialize, Deserialize, Clone)]
pub struct EnvState {
    pub node: NodeInfo,
    pub npm: NpmInfo,
    pub dsh: DshInfo,
    pub dsh_home: DshHomeInfo,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct NodeInfo {
    pub installed: bool,
    pub version: Option<String>,    // "24.14.1"
    pub path: Option<String>,
    pub meets_minimum: Option<bool>, // ≥ 22.0.0
}

#[derive(Serialize, Deserialize, Clone)]
pub struct NpmInfo {
    pub installed: bool,
    pub version: Option<String>,
    pub path: Option<String>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct DshInfo {
    pub installed: bool,
    pub version: Option<String>,    // "0.1.0-rc.6"
    pub path: Option<String>,
    pub latest_version: Option<String>,
    pub update_available: bool,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct DshHomeInfo {
    pub exists: bool,
    pub path: String,
    pub profiles_dir: bool,
    pub sessions_dir: bool,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct UpdateInfo {
    pub current_version: String,
    pub latest_version: String,
    pub update_available: bool,
    pub release_notes: Option<String>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct PluginRepo {
    pub full_name: String,          // "owner/repo"
    pub description: Option<String>,
    pub html_url: String,
    pub stargazers_count: u32,
    pub topics: Vec<String>,
    pub updated_at: String,
    pub owner_avatar: String,
    pub license: Option<String>,
    pub installed: bool,            // 交叉比对后标记
}

#[derive(Serialize, Deserialize, Clone)]
pub struct PluginSearchResult {
    pub repos: Vec<PluginRepo>,
    pub total_count: u32,
    pub page: u32,
    pub has_more: bool,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct InstalledPlugin {
    pub name: String,          // npm 包名
    pub version: String,
    pub path: Option<String>,
}
```

### 11.2 TypeScript 侧（类型同步）

```typescript
// src/lib/api/types.ts
export interface ProcessState { ... }
export interface EnvState { ... }
export interface NodeInfo { ... }
export interface DshInfo { ... }
export interface UpdateInfo { ... }
export interface PluginRepo { ... }
export interface PluginSearchResult { ... }
export interface InstalledPlugin { ... }
```

---

## 12. 错误处理

### 12.1 错误分类

| 错误类型 | 场景 | 用户可见行为 |
|----------|------|-------------|
| `NodeNotFound` | `node` 不在 PATH 中 | 环境面板显示 ❌ + 下载链接 |
| `NodeVersionTooLow` | Node < 22 | 环境面板显示 ⚠️ + 升级链接 |
| `DshNotFound` | `dsh` 未安装 | 环境面板显示 ❌ + 安装按钮 |
| `DshStartFailed` | `dsh web` 启动超时/崩溃 | 错误页面 + 重试按钮 + 日志 |
| `PortParseFailed` | stdout 解析失败 | 错误页面 + 显示原始 stdout 日志 |
| `InstallFailed` | npm install 失败 | 安装面板显示 stderr + 重试 |
| `NetworkError` | 无法访问 npm registry 或 GitHub API | 面板显示网络错误 |
| `PluginInstallFailed` | `dsh plugin add` 失败 | 插件面板显示 stderr + 重试 |

### 12.2 启动超时

```rust
const DSH_START_TIMEOUT: Duration = Duration::from_secs(30);

// 等待 stdout 中出现 URL 行，超时则报错
tokio::time::timeout(DSH_START_TIMEOUT, parse_url_from_stdout(...))
    .await
    .map_err(|_| "dsh web 启动超时 (30s)，请检查环境配置")?;
```

---

## 13. 安全考量

### 13.1 CSP 策略

Tauri 的 CSP 需要允许加载 `http://127.0.0.1:*`：

```json
{
  "app": {
    "security": {
      "csp": "default-src 'self'; connect-src http://127.0.0.1:* ws://127.0.0.1:* https://api.github.com https://registry.npmjs.org; script-src 'self' 'unsafe-inline'; style-src 'self' 'unsafe-inline'; img-src 'self' https://avatars.githubusercontent.com data:"
    }
  }
}
```

### 13.2 dsh 安全模型

dsh web 的安全限制由 upstream 设计：

- 默认绑定 `127.0.0.1`（仅本机访问）
- 故意拒绝 `--host 0.0.0.0`（防止网络暴露 RCE）
- 浏览器信任栅栏 (`trustedHosts`) 防止 DNS rebinding 攻击

App 不绕过这些限制，仅在本机 loopback 通信。

### 13.3 GitHub API 限流

GitHub Search API 未认证限流 60 次/小时。App 可选支持 GitHub Token 认证（提升至 5000 次/小时），Token 存储在 App 本地配置中。

---

## 14. 构建与分发

### 14.1 构建产物

| 平台 | 产物 | 说明 |
|------|------|------|
| Windows | `.msi` + `.exe` (NSIS) | 内含 WebView2 bootstrapper |
| macOS | `.dmg` | Universal Binary (x86_64 + aarch64) |
| Linux | `.AppImage` + `.deb` | 依赖 WebKitGTK |

### 14.2 Tauri 构建配置

```json
{
  "build": {
    "beforeDevCommand": "pnpm dev",
    "beforeBuildCommand": "pnpm build",
    "devUrl": "http://localhost:5173",
    "frontendDist": "../build"
  },
  "bundle": {
    "active": true,
    "targets": "all",
    "icon": ["icons/icon.ico", "icons/icon.png"],
    "windows": {
      "webviewInstallMode": {
        "type": "downloadBootstrapper"
      }
    }
  }
}
```

### 14.3 CI/CD (GitHub Actions)

```yaml
# .github/workflows/release.yml
- 自动构建三平台产物
- 签名 (macOS: notarization, Windows: code signing)
- 发布到 GitHub Releases
- 生成 latest.json (Tauri updater 端点)
```

---

## 15. 开发前置条件

### 15.1 开发机环境

| 工具 | 最低版本 | 安装方式 |
|------|----------|----------|
| Rust | 1.75+ | https://rustup.rs |
| Node.js | 22+ | https://nodejs.org 或 nvm |
| pnpm | 9+ | `npm i -g pnpm` |
| dsh | 0.1.0-rc.6+ | `npm i -g @deepseek-ai/dsh` |
| Tauri CLI | 2.0+ | `cargo install tauri-cli` 或 `pnpm add -D @tauri-apps/cli` |

### 15.2 首次开发环境搭建

```bash
# 1. 安装 Rust
# Windows: 下载 rustup-init.exe
# macOS/Linux: curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# 2. 安装前端依赖
pnpm install

# 3. 开发模式运行
pnpm tauri dev

# 4. 构建生产版本
pnpm tauri build
```

---

## 16. 路线图

### Phase 1 — MVP (当前)
- [x] 技术文档
- [ ] Tauri 项目脚手架
- [ ] dsh 进程管理 (spawn + stdout 解析 + kill)
- [ ] 主窗口 webview 加载
- [ ] 环境检查面板 (Node.js + dsh)
- [ ] dsh 安装引导

### Phase 2 — 完善
- [ ] 版本更新检查 (dsh + App 自身)
- [ ] 插件市场面板 (GitHub API 浏览 + 搜索)
- [ ] 插件安装/移除 (dsh plugin 封装)
- [ ] 插件激活 (cordis.patch.yml 编辑)
- [ ] 多语言支持 (i18n)
- [ ] 日志查看面板

### Phase 3 — 增强
- [ ] 开机自启动
- [ ] 系统托盘 (最小化到托盘)
- [ ] dsh profile 管理 UI
- [ ] API Key 配置 UI
- [ ] CI/CD 自动发布