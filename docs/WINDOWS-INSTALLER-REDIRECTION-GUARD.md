# Windows 安装包引导启动失败：RedirectionGuard 拦截 junction

> 排查记录 · 2026-08-17 · 影响版本 v1.1.3 ~ v1.1.5

## 现象

通过 Inno Setup 安装包安装完成后，勾选「运行 DSH Agent」直接启动 → dsh 服务必然启动失败。
关掉窗口，从开始菜单再打开同一个 exe → 一切正常。

```
Error: dsh: plugin tree failed to load: failed to apply loader entry include (cordis:include)
Error [ERR_MODULE_NOT_FOUND]: Cannot find package '@deepseek-ai/cordis-plugin-timer'
  imported from C:\Users\<user>\.dsh\profiles\web\
```

同批报错还涉及 `@deepseek-ai/dsh-llm`、`dsh-session`、`dsh-typert-registry`、
`dsh-typert-loader`、`dsh-api-gateway` 等。

关键的诡异之处：**同一个二进制、同一台机器、磁盘上没有任何变化**，只是换了个启动方式，
结果就不同。报错说包找不到，但那些包一直好端端地躺在磁盘上。

## 结论

Inno Setup 自身以 **RedirectionGuard enforcing 模式**运行，它用 `[Run]` 段启动的子进程会
**继承**这个进程缓解策略。而 dsh 的 profile 插件全部要穿过普通用户创建的 **junction（目录联接）**
才能解析到——RedirectionGuard enforcing 的作用恰恰就是**拒绝跟随非管理员创建的 junction**。

于是 Node 解析 `@deepseek-ai/*` 时穿不过去，报"包不存在"。

**与权限无关**（下方实验用 `PrivilegesRequired=lowest` 也照样复现），
**与提权令牌无关**，**与竞态无关**。

## 为什么 dsh 依赖 junction

dsh 的 profile 目录结构：

```text
~/.dsh/profiles/
├── node_modules/            ← fallback，全是 junction
│   └── @deepseek-ai/        ← 195 个条目，全部 LinkType=Junction
│       ├── cordis-plugin-timer  → D:\...\nvmd\versions\24.14.1\node_modules\
│       ├── dsh-llm              →   @deepseek-ai\dsh\node_modules\@deepseek-ai\<pkg>
│       └── ...
└── web/
    ├── package.json         ← "dependencies": {} —— 是空的！
    ├── cordis.yml           ← []，每次 boot 重写
    ├── cordis.patch.yml
    └── pnpm-workspace.yaml
    (没有 node_modules)
```

`profiles/web/package.json` 的 `dependencies` 为空，profile 目录下也没有自己的 `node_modules`。
插件是靠 **Node 逐级向上查找 `node_modules`** 命中 `profiles/node_modules` 这个 fallback 的：

```text
~/.dsh/profiles/web/node_modules      (不存在)
~/.dsh/profiles/node_modules          ← 命中，但里面是 195 个 junction
~/.dsh/node_modules                   (不存在)
...
```

这些 junction 由 dsh 在普通用户上下文（medium IL）下创建，属主是登录用户而非
Administrators —— 这是它们被判定为"不可信挂载点"的直接原因。

验证方式：

```powershell
$fb = "$env:USERPROFILE\.dsh\profiles\node_modules\@deepseek-ai"
Get-ChildItem $fb -Force | Group-Object LinkType    # → Junction : 195
(Get-Acl (Join-Path $fb 'cordis-plugin-timer')).Owner   # → <机器名>\<用户名>
```

## 什么是 RedirectionGuard

Windows 的进程缓解策略之一（`ProcessRedirectionTrustPolicy`），用于防止特权进程被
低权限用户埋下的链接重定向到非预期位置（一类经典提权手法）。开启 enforcing 后，
该进程**拒绝跟随非管理员创建的 junction / mount point**，访问时返回
`ERROR_UNTRUSTED_MOUNT_POINT`（0x1396）。

**Inno Setup 6.3+ 默认为 Setup 进程开启此策略**，且它启动的子进程会继承。
在 Setup 日志（`SetupLogging=yes` → `%TEMP%\Setup Log *.txt`）第 13 行可以看到：

```
RedirectionGuard status for current process: Enabled in enforcing mode
```

注意这个策略是**单向**的：进程可以把自己的缓解策略收紧，但**无法放松**。
所以已经继承到 enforcing 的进程救不回来，只能从一开始就不要继承。

## 决定性实验

一个不装任何文件、不建目录、不写卸载项的一次性 Inno 脚本，
`[Run]` 段直接 Exec 一条读 junction 的命令：

```ini
[Setup]
AppName=RGProbe
AppVersion=1.0
CreateAppDir=no
Uninstallable=no
PrivilegesRequired=lowest        ; 注意：最低权限，排除提权因素
DisableProgramGroupPage=yes
DisableReadyPage=yes
DisableFinishedPage=yes
OutputDir=<tmp>
OutputBaseFilename=rgprobe

[Run]
Filename: "{cmd}"; Parameters: "/c (echo [dir] & dir ""<LINK>"" & echo [type] & type ""<LINK>\package.json"") > ""<tmp>\out.txt"" 2>&1"; Flags: runhidden waituntilterminated runasoriginaluser
```

其中 `<LINK>` = `C:\Users\<user>\.dsh\profiles\node_modules\@deepseek-ai\cordis-plugin-timer`。

以 `/VERYSILENT /SUPPRESSMSGBOXES` 运行，`out.txt` 内容：

```text
[dir]
 Directory of C:\Users\<user>\.dsh\profiles\node_modules\@deepseek-ai\cordis-plugin-timer

File Not Found
[type]
The path cannot be traversed because it contains an untrusted mount point.
```

同一条命令在普通 PowerShell / cmd 里执行：

```text
2026/08/14  09:47               867 package.json      (exit=0)
```

**这就是全部真相**：Setup 的子进程"看不见"这些包，普通进程看得见。
`PrivilegesRequired=lowest` + `runasoriginaluser` 都用上了仍然复现，
证明与权限、与令牌完全无关。

## 修复

`installer.iss` 的 `[Run]` 不再直接 Exec 本程序，改为经 `explorer.exe` 转交桌面 shell：

```ini
[Run]
Filename: "{win}\explorer.exe"; Parameters: """{app}\dsh-agent.exe"""; \
  Description: "{cm:LaunchProgram,DSH Agent}"; \
  Flags: nowait postinstall skipifsilent runasoriginaluser
```

`explorer.exe <path>` 会把请求转交给已在运行的桌面 shell，最终进程挂在 explorer 下
**而不是 Setup 下**，于是令牌、缓解策略、环境块、句柄全部与 Setup 脱钩。

选这个改法而不是"想办法关掉 RedirectionGuard"，有两个理由：

1. 缓解策略无法放松，技术上做不到；
2. 它切断的是与 Setup 的**全部**继承关系。即使日后发现真凶另有其人（继承的环境块、
   `__COMPAT_LAYER`、句柄……），这个改动一样有效——不依赖根因判断正确。

修复后日志（经 explorer 启动）：

```text
launch context: cwd=C:\WINDOWS\system32              ← Setup 直接 Exec 时会是安装目录
launch context: RedirectionGuard enforce=0 audit=0 (raw=0x00000000)
launch context: read ...\cordis-plugin-timer\package.json OK (867 bytes)
dsh stdout: dsh web: http://127.0.0.1:xxxxx
```

## 诊断留档

`src-tauri/src/cmd_ext.rs::log_launch_context()` 在每次 `start_dsh` 前记录：

- 当前工作目录（能反推出启动来源）
- 本进程 RedirectionGuard 的 `enforce` / `audit` 位（`GetProcessMitigationPolicy`）
- 实际读一次 `profiles/node_modules/@deepseek-ai/cordis-plugin-timer/package.json`

以后再遇到"同一个 exe 换个启动方式就挂"，先看这三行，不要猜。

> 实现注记：`windows-sys 0.59` 有 `ProcessRedirectionTrustPolicy` 常量但没有
> `PROCESS_MITIGATION_REDIRECTION_TRUST_POLICY` 类型。该类型是 DWORD 与位域在同 4 字节上的
> 联合体，直接用 `u32` 接收即字节等价，无需额外依赖。

## 走过的弯路

两个被实验推翻的"根因"，记下来是为了不再走第三遍。

### ❌ 弯路一：以为是"dsh 重建符号链接与 boot 竞态"

**主张**：切换权限上下文后 dsh 会重建 `profiles/node_modules`，重建没完成就 boot，所以找不到包。
**处置**：启动失败自动重试 3 次，每次重试前 wipe `profiles/node_modules`。

**为什么错**：失败期间 `profiles/node_modules` 的 mtime **始终停在 08:28:46 没变过**——
根本没有发生任何重建。

**而且有害**：首次失败时 fallback 目录本是完好的，wipe 反而删掉 252 个条目逼 dsh 从零重建，
把「再跑一次就好」变成「三次全灭」。

**判据**：主张"某个目录正在被重建"时，先去看那个目录的 mtime。

### ❌ 弯路二：以为是 `[Run]` 缺 `runasoriginaluser`

**主张**：`PrivilegesRequired=admin` 下 Inno 让被启动程序继承管理员令牌，与手动启动上下文不同。
**处置**：`[Run]` 加 `runasoriginaluser`。

**为什么错**：该 flag 对 `postinstall` 条目**本来就是默认行为**。对比加 flag 前后两份 Setup 日志：

| Setup 日志 | Inno 版本 | 安装包 | `Run as:` |
|---|---|---|---|
| `#004` @08:12 | 6.7.1 | 加 flag **之前** | `Original user` |
| `#006` @09:46 | 6.7.3 | 加 flag **之后** | `Original user` |

这行**加之前就已经在打印了**，改动是彻底的 no-op。真机重装验证也确认没修好。

**判据**：改 flag 之前，先在日志里找这个 flag 应该影响的那行输出，确认它当前是什么值。

### ✅ 真正定位问题的方法

1. **把两边日志的时间戳逐条对齐**。把 Setup 日志的 `-- Run entry --` 时间戳和应用日志的
   进程启动时间戳对上（实测相差 76ms），才发现之前被当成"手动启动也失败"的反例
   其实是引导启动。修正后相关性变成 **引导 5/5 失败、手动 5/5 成功，无一反例**。
2. **在"同一个二进制、同一份磁盘状态、不同结果"面前，唯一的变量只能是进程上下文**。
   顺着这条往下想，才会去读 Setup 日志里那行一直在眼皮底下的 RedirectionGuard。
3. **做一个能独立复现的最小实验**，而不是改完真机重装一次赌对错。

## 对其他项目的普遍意义

任何满足以下条件的 Windows 应用都会踩到：

- 用 Inno Setup 6.3+ 打包，且用 `[Run]` 的"安装完成后启动"复选框
- 运行时需要穿过普通用户创建的 junction / symlink

典型场景：pnpm 的 store 硬链接方案、Node 版本管理器（nvm / nvmd / fnm）的 shim 目录、
各类 `node_modules` 提升/软链方案、Scoop 的 `current` 联接。

**通用对策**：`[Run]` 一律经 `explorer.exe` 转交，不要让 Setup 直接 Exec 你的应用。
