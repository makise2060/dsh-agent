//! 下载源清单与降级顺序。
//!
//! 国内直连 nodejs.org / registry.npmjs.org 经常超时甚至不通，
//! 所以镜像放在前面，官方源只作兜底。

pub struct Mirror {
    pub name: &'static str,
    pub base: &'static str,
}

/// Node.js 官方源。
/// 校验和只认它：哈希若和 zip 取自同一个镜像，校验就只能挡住传输损坏，
/// 挡不住镜像本身作恶 —— 它可以同时给出改过的包和一份匹配的哈希。
pub const NODE_OFFICIAL: &str = "https://nodejs.org/dist";

/// Node.js 二进制。三个源的目录结构完全一致：
/// `{base}/index.json` 与 `{base}/v{VER}/node-v{VER}-win-x64.zip`
///
/// 顺序只是兜底基线：实际尝试顺序由 download.rs 的 rank_mirrors 在运行时
/// 测速决定 —— 写死的顺序表达不了「镜像活着但被限速」这种当下状态。
pub const NODE_MIRRORS: &[Mirror] = &[
    Mirror {
        name: "阿里云 npmmirror",
        base: "https://cdn.npmmirror.com/binaries/node",
    },
    Mirror {
        name: "清华 TUNA",
        base: "https://mirrors.tuna.tsinghua.edu.cn/nodejs-release",
    },
    Mirror {
        name: "Node.js 官方",
        base: NODE_OFFICIAL,
    },
];

/// npm registry。安装 dsh 与插件时通过 `--registry` / `npm_config_registry`
/// 临时指定，不写用户的全局 npm 配置。
pub const NPM_REGISTRIES: &[Mirror] = &[
    Mirror {
        name: "阿里云 npmmirror",
        base: "https://registry.npmmirror.com",
    },
    Mirror {
        name: "npm 官方",
        base: "https://registry.npmjs.org",
    },
];
