<p align="center">
  <img src="assets/dsh-agent-logo.png" width="200" alt="DSH Agent Logo" />
</p>

<h1 align="center">DSH Agent</h1>

<p align="center">
  DeepSeek Harness 桌面 Webview 包装器 — 一键启动 <code>dsh web</code>，在原生窗口中直接使用。
</p>

<p align="center">
  <a href="https://v2.tauri.app"><img src="https://img.shields.io/badge/Tauri-2.x-orange" alt="Tauri" /></a>
  <a href="https://kit.svelte.dev"><img src="https://img.shields.io/badge/SvelteKit-2.x-ff3e00" alt="SvelteKit" /></a>
  <a href="https://www.rust-lang.org"><img src="https://img.shields.io/badge/Rust-edition%202021-dea584" alt="Rust" /></a>
  <a href="#license"><img src="https://img.shields.io/badge/license-MIT-blue" alt="License" /></a>
</p>

## 概述

DSH Agent 将 DeepSeek Harness 的 Web UI 包装为一个**独立桌面应用**。用户双击图标即可使用，无需打开终端、输入 `dsh web`、再手动在浏览器中访问 URL。

### 核心功能

| 功能 | 说明 |
|------|------|
| **一键启动** | 自动在后台 spawn `dsh web --port 0`，从 stdout 解析随机端口，在 webview 中加载 `http://127.0.0.1:{port}` |
| **环境自检** | 检测 Node.js（≥22）、npm、dsh 安装状态与路径，展示 `DSH_HOME` 目录结构 |
| **一键安装 dsh** | 未安装或版本过旧时，点击按钮执行 `npm install -g @deepseek-ai/dsh@latest`，stdout 实时流式输出 |
| **版本更新** | 通过 npm registry 查询 dsh 最新版本，semver 比较后提示更新 |
| **插件市场** | 调用 GitHub Search API 搜索 `topic:dsh-plugin` 仓库，支持安装 / 移除 / 激活插件 |
| **进程清理** | 窗口关闭时自动 `taskkill /T /F`（Windows）或 `kill -KILL`（Unix）终止 dsh 子进程树 |

## 技术栈

| 层 | 技术 | 说明 |
|---|---|---|
| 桌面框架 | **Tauri 2.x** | Rust 后端 + 系统 WebView2，安装包仅 ~5MB |
| 前端框架 | **SvelteKit 2.x** (静态 SPA) | 编译为纯静态文件，由 Tauri 内嵌加载 |
| UI 框架 | **Tailwind CSS 3.x** | 原子化 CSS，快速构建设置面板 |
| 后端 | **Rust** (edition 2021) | tokio async runtime，进程管理 + HTTP 请求 |
| 包管理 | **pnpm** + **Cargo** | 前端依赖用 pnpm，Rust 依赖用 Cargo |

## 项目结构

```
dsh-agent/
├── src/                          # 前端 (SvelteKit)
│   ├── app.css / app.html        # 全局样式与 HTML 入口
│   ├── lib/
│   │   ├── api/
│   │   │   ├── types.ts          # 前后端共享类型定义
│   │   │   └── tauri.ts          # Tauri invoke 封装
│   │   ├── components/            # 通用组件
│   │   │   ├── TopNav.svelte      # 顶部导航栏
│   │   │   ├── LoadingScreen.svelte
│   │   │   └── StatusBadge.svelte
│   │   ├── stores/               # Svelte stores
│   │   └── views/                # 四个主视图
│   │       ├── Main.svelte        # iframe 加载 dsh web
│   │       ├── EnvPanel.svelte    # 运行环境检查
│   │       ├── VersionPanel.svelte # 版本更新
│   │       └── PluginMarket.svelte # 插件市场
│   └── routes/+layout.svelte     # 路由入口
├── src-tauri/                    # Rust 后端
│   ├── Cargo.toml
│   ├── tauri.conf.json           # Tauri 窗口/CSP/打包配置
│   ├── icons/                   # 全平台图标
│   └── src/
│       ├── main.rs / lib.rs      # 入口 + 命令注册
│       ├── state.rs              # tokio::sync::Mutex 状态管理
│       ├── process.rs            # dsh 进程 spawn / 端口解析 / kill
│       ├── env.rs                # Node/npm/dsh 版本检测
│       ├── installer.rs          # npm install -g 安装引导
│       ├── version.rs            # 版本更新检查
│       └── plugins.rs            # GitHub API 插件市场
├── docs/
│   ├── DESIGN.md                # 详细技术设计文档
│   └── DSH-ANALYSIS.md          # dsh 上游分析文档
└── package.json
```

## 快速开始

### 前置要求

| 工具 | 最低版本 | 说明 |
|------|---------|------|
| [Node.js](https://nodejs.org/) | ≥ 22 | dsh 本身依赖 Node.js 运行时 |
| [pnpm](https://pnpm.io/) | ≥ 9 | 前端包管理器 |
| [Rust](https://www.rust-lang.org/) | ≥ 1.75 | Tauri 后端编译 |
| [dsh](https://www.npmjs.com/package/@deepseek-ai/dsh) | latest | `npm install -g @deepseek-ai/dsh` |

> **Windows 用户**：需安装 [WebView2 Runtime](https://developer.microsoft.com/microsoft-edge/webview2/)（Win11 已内置）。
>
> **MinGW 工具链注意**：如果使用 `x86_64-pc-windows-gnu` 工具链，`Cargo.toml` 中的 `crate-type` 不应包含 `cdylib`（MinGW 链接器有符号导出数量限制），当前配置已默认为 `["staticlib", "rlib"]`。

### 安装依赖

```bash
# 前端依赖
pnpm install

# Rust 依赖会在首次构建时自动拉取
```

### 开发模式

```bash
pnpm tauri dev
```

启动后：
1. Vite dev server 在 `http://localhost:5173` 运行
2. Cargo 编译 Rust 后端（首次约 5-8 分钟）
3. Tauri 窗口弹出，WebView 加载前端
4. 前端调用 `start_dsh` 命令，Rust 后端 spawn `dsh web --port 0`
5. 从 stdout 解析端口后，iframe 加载 `http://127.0.0.1:{port}`

### 生产构建

```bash
pnpm tauri build
```

生成物位于 `src-tauri/target/release/bundle/`：
- Windows: `.msi` / `.exe` 安装包
- macOS: `.dmg` / `.app`
- Linux: `.deb` / `.AppImage`

## 界面说明

应用顶部导航栏提供四个功能页签：

### 主界面

启动 `dsh web` 并在 iframe 中加载。右上角状态指示器显示当前进程状态：

| 状态 | 含义 |
|------|------|
| 未启动 | dsh 进程尚未启动 |
| 启动中 | 正在 spawn 进程并等待端口输出 |
| 运行中 | dsh web 已在运行，iframe 已加载 |
| 停止中 | 正在终止进程树 |
| 已停止 | 进程已终止 |
| 错误 | 启动失败（超时 / dsh 未安装等） |

### 运行环境

检测并展示：
- **Node.js** — 版本号、安装路径、是否满足 ≥22 要求
- **npm** — 版本号、安装路径
- **dsh** — 版本号、安装路径、最新版本、是否有更新
- **DSH_HOME** — 目录是否存在、`profiles/` 和 `sessions/` 子目录状态

未安装 dsh 时提供「一键安装」按钮。

### 版本更新

- **DSH Agent** — 应用自身版本检查（接入 Tauri Updater）
- **dsh** — 通过 npm registry 查最新版本，semver 比较后提示更新

### 插件市场

- 调用 GitHub Search API 搜索 `topic:dsh-plugin` 仓库
- 支持按 Stars / 最近更新 / 名称 排序
- 关键词搜索
- 每个仓库卡片显示：名称、描述、Star 数、最近更新时间、License
- 已安装的插件显示「移除」按钮，未安装的显示「安装」按钮
- 安装通过 `dsh plugin --profile web add <package>` 执行，stdout 实时流式显示

## Rust 后端命令

所有前端 → 后端调用通过 Tauri IPC (`invoke`)：

| 命令 | 参数 | 返回值 | 说明 |
|------|------|--------|------|
| `start_dsh` | — | `ProcessState` | spawn `dsh web --port 0`，解析 stdout 获取端口 |
| `stop_dsh` | — | `()` | 终止 dsh 进程树 |
| `get_dsh_status` | — | `ProcessState` | 查询当前进程状态 |
| `restart_dsh` | — | `ProcessState` | 先 stop 再 start |
| `check_environment` | — | `EnvState` | 检测 Node/npm/dsh/DSH_HOME |
| `install_dsh` | — | `()` | `npm install -g @deepseek-ai/dsh@latest` |
| `check_dsh_update` | — | `UpdateInfo` | 查 npm registry 比较版本 |
| `check_app_update` | — | `UpdateInfo` | 应用自身版本检查 |
| `search_plugins` | `query?, sort?, page?` | `PluginSearchResult` | GitHub API 搜索 |
| `list_installed_plugins` | — | `InstalledPlugin[]` | 列出已安装插件 |
| `install_plugin` | `packageName` | `()` | `dsh plugin add` |
| `remove_plugin` | `packageName` | `()` | `dsh plugin remove` |
| `activate_plugin` | `pluginId, pluginName` | `()` | 编辑 `cordis.patch.yml` |

### 事件

| 事件名 | Payload | 说明 |
|--------|---------|------|
| `process-state-changed` | `ProcessState` | 进程状态变更通知 |
| `install-progress` | `{ stage, message, percent? }` | dsh 安装进度 |
| `plugin-install-progress` | `{ package, stage, message }` | 插件安装进度 |

## 配置

### Tauri 配置 (`src-tauri/tauri.conf.json`)

关键配置项：
- **窗口**：1280×800，最小 900×600，可调整大小
- **CSP**：允许 `127.0.0.1:*` 的 connect/frame（dsh web）、`api.github.com`（插件搜索）、`registry.npmjs.org`（版本检查）、`avatars.githubusercontent.com`（头像图片）
- **WebView2**：缺失时自动下载 Bootstrapper 安装

### DSH_HOME

dsh 的配置目录，默认为 `~/.dsh/`，可通过环境变量 `DSH_HOME` 自定义：
```
~/.dsh/
├── profiles/
│   └── web/
│       ├── package.json      # 插件依赖
│       ├── node_modules/     # 已安装插件
│       └── cordis.patch.yml   # 插件激活配置
└── sessions/                 # 会话数据
```

## 开发指南

### 前端开发

```bash
# 仅启动前端 dev server（不启动 Tauri 窗口）
pnpm dev

# 类型检查
pnpm check
```

前端使用 Svelte 5 runes 语法，路由通过自定义 store（`currentRoute`）管理，不走 SvelteKit 文件路由。

### Rust 后端开发

Rust 后端代码在 `src-tauri/src/` 下，修改后会自动触发 Tauri 重新编译。

状态管理使用 `tokio::sync::Mutex`（而非 `std::sync::Mutex`），因为 Tauri 命令是 async 的，MutexGuard 需要跨 `.await` 持有，必须满足 `Send`。

### 添加新的 Tauri 命令

1. 在 `src-tauri/src/` 对应模块中添加 `#[tauri::command]` 函数
2. 在 `src-tauri/src/lib.rs` 的 `invoke_handler` 中注册
3. 在 `src/lib/api/tauri.ts` 中添加前端调用封装
4. 在 `src/lib/api/types.ts` 中添加类型定义

## 技术文档

- [DESIGN.md](./docs/DESIGN.md) — 完整技术设计文档（架构、进程管理、插件市场、安全策略）
- [DSH-ANALYSIS.md](./docs/DSH-ANALYSIS.md) — dsh 上游产品分析（启动链路、配置层叠、目录结构）

## 关于

<table>
  <tr>
    <td width="120" align="center">
      <img src="https://avatars.githubusercontent.com/u/263516586?v=4" width="100" height="100" style="border-radius: 50%;" alt="makise2060" />
    </td>
    <td>
      <h3>DrPepper (<a href="https://github.com/makise2060">@makise2060</a>)</h3>
      <p>山东科技大学 · 计算机科学与技术</p>
      <p>Makise(Sometimes) — an ordinary backend engineer navigating the AI tide. Optimistic, rational, and full of hope, building wonders in everyday life.</p>
      <p>
        <a href="https://github.com/makise2060"><img src="https://img.shields.io/badge/GitHub-makise2060-181717?logo=github&logoColor=white" alt="GitHub" /></a>
      </p>
    </td>
  </tr>
</table>

## 致谢

本项目的诞生离不开 [Linux.do](https://linux.do/) 社区的启发。感谢社区中分享 DeepSeek Harness 使用经验、推荐 Tauri 方案、讨论进程管理细节的各位大佬——是你们的真诚、友善、团结、专业让开源社区充满活力。

## License

MIT
