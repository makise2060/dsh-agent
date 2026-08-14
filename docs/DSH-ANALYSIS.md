# DeepSeek Harness (dsh) 技术分析

> 本文档是对 DeepSeek Harness 上游产品的逆向分析，作为 DSH Agent 项目的设计依据。

## 1. 产品概览

| 维度 | 值 |
|------|-----|
| **npm 包名** | `@deepseek-ai/dsh` |
| **当前版本** | `0.1.0-rc.6` |
| **发布日期** | 2026-08-13 |
| **维护者** | imccyu, tianyicui-deepseek |
| **仓库** | https://github.com/deepseek-ai/deepseek-harness |
| **许可证** | MIT |
| **框架基础** | Cordis (plugin loader / dependency injection) |
| **Node.js 要求** | ≥ 22 (使用 `node:sqlite` 等内置模块) |

## 2. CLI 命令体系

### 2.1 入口命令

```
dsh [options] [command] [args...]
```

| 命令 | 用途 |
|------|------|
| `dsh --profile <name>` | 启动指定 profile |
| `dsh web` | `--profile web` 别名，启动浏览器 UI |
| `dsh --profile headless "job"` | 单次任务执行并退出 |
| `dsh plugin --profile <name> <pnpm args>` | 管理 profile 的插件依赖 (转发给 pnpm) |

### 2.2 全局选项

| 选项 | 说明 |
|------|------|
| `-V, --version` | 输出版本号 |
| `--profile <name>` | 指定 profile (必需) |
| `--patch <path>` | 额外 patch 覆盖层 (可重复) |
| `--dump-config` | 打印组合后的配置树并退出 |
| `--dump-default-config` | 打印不含用户层的默认配置树 |

### 2.3 `dsh web` 子命令

```
Usage: dsh --profile web [options]

Serve the DeepSeek Harness browser UI.

Options:
  --host <host>                  bind host (仅接受 127.0.0.1)
  --port <port>                  listen port; 0 让 OS 自动分配
  --trusted-host <authority...>  额外信任的 authority (可重复)
  -h, --help                     显示帮助
```

**安全限制**：`--host 0.0.0.0` 被故意拒绝，防止远程代码执行暴露到网络。

## 3. Profile 与 Plugin Bundle 架构

### 3.1 层叠组合模型

dsh 的配置不是单一文件，而是多层 patch 按顺序叠加：

```
空根配置 ([] )
  ↓
bundle 层 1: @deepseek-ai/dsh-base     (基础 agent 能力)
  ↓
bundle 层 2: @deepseek-ai/dsh-web-app   (Web 表层补丁)
  ↓
profile 层: $DSH_HOME/profiles/web/cordis.patch.yml
  ↓
home 层: $DSH_HOME/cordis.patch.yml     (机器级全局偏好)
  ↓
overlay 层: --patch 参数指定的额外覆盖
```

每一层是一个 YAML patch 列表，通过 id 覆盖、禁用或插入配置行。

### 3.2 Web Profile 的 bundle 组合

`$DSH_HOME/profiles/web/package.json`:

```json
{
  "name": "dsh-profile-web",
  "private": true,
  "dependencies": {},
  "dsh": {
    "profile": {
      "bundles": [
        "@deepseek-ai/dsh-base",
        "@deepseek-ai/dsh-web-app"
      ]
    }
  }
}
```

- `dsh-base`：提供 agent 核心 (LLM、session、sandbox、tools、skill 系统、subagent 等)
- `dsh-web-app`：在 base 之上叠加 Web 表层 (webserver、前端 dist 服务、API gateway、浏览器端 UI 插件名录)

### 3.3 插件管理

dsh 的插件通过 `dsh plugin` 命令管理，实际是转发给 pnpm：

```bash
# 安装插件到 web profile
dsh plugin --profile web add @some-org/dsh-plugin-xxx

# 移除插件
dsh plugin --profile web remove @some-org/dsh-plugin-xxx

# 查看已安装
dsh plugin --profile web list
```

插件安装在 `$DSH_HOME/profiles/web/node_modules/` 下，通过 `cordis.patch.yml` 激活。

## 4. Web Server 实现分析

### 4.1 启动链路

```
dsh web --port 0
  │
  ├─ bin.js: 解析命令行 → resolveBoot("web")
  │
  ├─ profile-boot.js: composeProfile("web")
  │   ├─ 加载 dsh-base bundle patches
  │   ├─ 加载 dsh-web-app bundle patches
  │   ├─ 加载用户 cordis.patch.yml
  │   └─ 组合为最终配置树
  │
  ├─ boot(): 初始化 Cordis 插件树
  │
  ├─ web-startup 插件: 解析 --host/--port/--trusted-host
  │   └─ provide webStartup 服务
  │
  ├─ webserver 插件 (dsh-host-webserver):
  │   ├─ inject webStartup → 读取 host/port
  │   ├─ createServer (node:http)
  │   ├─ server.listen(port, host)
  │   └─ 提供 webServer 服务 (路由注册、fallback、upgrade)
  │
  ├─ web-runtime 插件 (dsh-web-app):
  │   ├─ inject webServer → 获取实际绑定端口
  │   ├─ resolveDistIndex() → require.resolve("@deepseek-ai/dsh-web-frontend/dist/index.html")
  │   ├─ mount FrontendStatic (SPA 静态文件 fallback)
  │   ├─ 注册 web-surface prompt section
  │   ├─ 注册 DSH_WEB_URL shell 环境变量
  │   └─ printUrl: console.log("dsh web: http://127.0.0.1:<port>")
  │
  └─ 前端插件名录 (dsh.client rows):
      ├─ modules → 扫描配置树 → window.__DSH_BOOT__
      ├─ connection → /api fetch/SSE client
      ├─ ui-* → 浏览器端 UI 组件 (conversation, sidebar, settings, ...)
      └─ cordis-client-runner → 浏览器端 Cordis 运行时
```

### 4.2 HTTP Server (dsh-host-webserver)

基于 Node.js 原生 `node:http`：

- **路由系统**：exact route + prefix route (最长前缀匹配) + fallback seat
- **WebSocket upgrade**：按 pathname 注册 upgrade handler
- **index.html transform taps**：fallback handler 渲染 index.html 时按注册顺序应用 transform (用于注入 `window.__DSH_BOOT__`)
- **默认绑定**：`127.0.0.1:3080`
- **端口分配**：传 `--port 0` 时由 OS 分配，通过 `server.address().port` 获取实际端口

### 4.3 前端静态文件服务 (dsh-host-frontend-static)

- **dist 位置**：`@deepseek-ai/dsh-web-frontend/dist/index.html` (通过 require.resolve 定位)
- **SPA 模式**：未匹配路径 fallback 到 index.html (HTTP 200)
- **安全**：目录穿越检测 (resolve 后检查是否在 dist root 内)，越界返回 403
- **MIME 类型**：.html, .js, .css, .svg, .json, .map, .webmanifest，其余 octet-stream
- **方法限制**：仅 GET/HEAD，其余 405

### 4.4 stdout 输出格式

```
dsh web: http://127.0.0.1:<port>
```

带 LAN 时 (仅当 `--host 0.0.0.0` 时才有，当前版本拒绝此 host)：

```
dsh web: http://127.0.0.1:<port> (LAN: http://<lan-ip>:<port>)
```

### 4.5 前端 dist 结构

```
@deepseek-ai/dsh-web-frontend/dist/
├── assets/              # 编译后的 JS/CSS chunks
├── favicon.svg
├── index.html           # SPA 入口 (经过 index taps 注入 __DSH_BOOT__)
└── manifest.webmanifest # PWA manifest
```

## 5. DSH_HOME 目录结构

```
~/.dsh/                           ($DSH_HOME, 默认 ~/.dsh)
├── .credentials.yaml            # API keys (DEEPSEEK_API_KEY, FAST_MODEL_API_KEY 等)
├── settings.yaml                # 用户全局设置 (模型配置、主题等)
├── cordis.patch.yml             # 机器级全局 patch 层
├── profiles/
│   ├── web/                     # Web profile
│   │   ├── package.json         # bundle 声明 + 插件依赖
│   │   ├── cordis.yml           # 空根配置 (每次启动重写)
│   │   ├── cordis.patch.yml     # 用户 profile 级 patch
│   │   ├── pnpm-workspace.yaml  # pnpm 工作区配置
│   │   └── node_modules/        # pnpm 安装的插件包
│   └── cc-tui/                  # 其他自定义 profile
├── sessions/                    # 持久化会话 (JSONL 格式)
├── storages/                    # JSON 存储 (dsh-storage-json)
└── .agent-presets/              # 用户自定义 agent presets (可写)
```

### 5.1 settings.yaml 示例

```yaml
ui-onboarding:
  welcomeNoticeVersion: 2026-08-13.1
llm-deepseek: {}
llm-pi-ai:
  providers:
    fast-model:
      apiKeyEnv: FAST_MODEL_API_KEY
      api: openai-responses
      baseURL: https://fastmodel.top/v1
      models:
        - id: deepseek-v4-flash
          contextWindow: 1000000
          reasoningEfforts: { off: none, low: low, medium: medium, high: high, xhigh: xhigh, max: max }
        - id: deepseek-v4-pro
          contextWindow: 1000000
          reasoningEfforts: { off: none, low: low, medium: medium, high: high, xhigh: xhigh, max: max }
agent-default-model:
  provider: fast-model
  model: deepseek-v4-pro
  reasoningEffort: high
ui-theme:
  preference: light
```

### 5.2 .credentials.yaml 示例

```yaml
DEEPSEEK_API_KEY: sk-xxxx
FAST_MODEL_API_KEY: sk-yyyy
```

## 6. 平台差异

dsh 内置了平台感知：

| 维度 | Windows | macOS / Linux |
|------|---------|---------------|
| Shell sandbox | `dsh-pwsh-sandbox` (启用) | `dsh-bash-sandbox` (启用) |
| 互补 sandbox | `dsh-bash-sandbox` (disabled) | `dsh-pwsh-sandbox` (disabled) |

配置中通过 `!!js process.platform` 条件禁用实现。

## 7. 插件生态 (GitHub Topic: dsh-plugin)

GitHub 上 `dsh-plugin` topic 下的仓库可通过 GitHub Search API 获取：

```
GET https://api.github.com/search/repositories?q=topic:dsh-plugin&sort=stars&order=desc
```

返回标准 GitHub Repo JSON (total_count ~834+)，包含 `full_name`, `description`, `html_url`, `stargazers_count`, `topics`, `updated_at` 等字段。

### 7.1 dsh 自带的插件管理

- **安装**：`dsh plugin --profile web add <package>` → pnpm add
- **移除**：`dsh plugin --profile web remove <package>` → pnpm remove
- **已安装查询**：`dsh plugin --profile web list` → pnpm list
- **运行时清单**：`pluginInventory/list` Remote (只读投影当前 Loader 树)

### 7.2 插件激活方式

安装包后，需要在 profile 的 `cordis.patch.yml` 中添加 patch 行来激活插件：

```yaml
- id: my-plugin-id
  name: '@some-org/dsh-plugin-xxx'
  config:
    # 插件特定配置
```

---

> **本文件是上游分析文档，不包含 DSH Agent 项目自身的设计。项目设计见 [DESIGN.md](./DESIGN.md)。**
