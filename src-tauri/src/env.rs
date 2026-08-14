use crate::cmd_ext::{hidden_command, silent_command};
use serde::{Deserialize, Serialize};
use semver::Version;
use std::path::PathBuf;
use tauri::State;

use crate::state::AppState;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct NodeInfo {
    pub installed: bool,
    pub version: Option<String>,
    pub path: Option<String>,
    pub meets_minimum: Option<bool>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct NpmInfo {
    pub installed: bool,
    pub version: Option<String>,
    pub path: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct DshInfo {
    pub installed: bool,
    pub version: Option<String>,
    pub path: Option<String>,
    pub latest_version: Option<String>,
    pub update_available: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct DshHomeInfo {
    pub exists: bool,
    pub path: String,
    pub profiles_dir: bool,
    pub sessions_dir: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct EnvState {
    pub node: NodeInfo,
    pub npm: NpmInfo,
    pub dsh: DshInfo,
    pub dsh_home: DshHomeInfo,
}

const NODE_MIN_VERSION: &str = "22.0.0";

#[cfg(target_os = "windows")]
fn find_executable(name: &str) -> tokio::process::Command {
    let mut cmd = hidden_command("where");
    cmd.arg(name);
    cmd
}

#[cfg(not(target_os = "windows"))]
fn find_executable(name: &str) -> tokio::process::Command {
    let mut cmd = hidden_command("which");
    cmd.arg(name);
    cmd
}

async fn get_version(cmd: &str, args: &[&str]) -> Option<String> {
    #[cfg(target_os = "windows")]
    {
        // On Windows, npm global installs are .cmd shims; use cmd.exe /C
        let mut full_args = vec!["/C", cmd];
        full_args.extend_from_slice(args);
        let output = silent_command("cmd.exe")
            .args(&full_args)
            .output()
            .await
            .ok()?;
        if !output.status.success() {
            return None;
        }
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if stdout.is_empty() {
            return None;
        }
        Some(stdout)
    }
    #[cfg(not(target_os = "windows"))]
    {
        let output = silent_command(cmd).args(args).output().await.ok()?;
        if !output.status.success() {
            return None;
        }
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if stdout.is_empty() {
            return None;
        }
        Some(stdout)
    }
}

async fn find_path(name: &str) -> Option<String> {
    let output = find_executable(name).output().await.ok()?;
    if !output.status.success() {
        return None;
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    stdout.lines().next().map(|s| s.to_string())
}

fn parse_node_version(raw: &str) -> Option<String> {
    let v = raw.trim().trim_start_matches('v');
    if v.is_empty() {
        return None;
    }
    Some(v.to_string())
}

fn parse_dsh_version(raw: &str) -> Option<String> {
    let v = raw.trim();
    // dsh version is like "0.1.0-rc.6"
    match Version::parse(v) {
        Ok(_) => Some(v.to_string()),
        Err(_) => {
            // Try to extract just the version number
            let parts: Vec<&str> = v.split_whitespace().collect();
            for part in parts {
                if let Ok(_) = Version::parse(part) {
                    return Some(part.to_string());
                }
            }
            None
        }
    }
}

fn check_node_meets_minimum(version: &str) -> bool {
    match Version::parse(version) {
        Ok(v) => v >= Version::parse(NODE_MIN_VERSION).unwrap(),
        Err(_) => false,
    }
}

pub fn get_dsh_home() -> PathBuf {
    if let Ok(custom) = std::env::var("DSH_HOME") {
        return PathBuf::from(custom);
    }
    dirs::home_dir()
        .map(|h| h.join(".dsh"))
        .unwrap_or_else(|| PathBuf::from("~/.dsh"))
}

#[tauri::command]
pub async fn check_environment(_state: State<'_, AppState>) -> Result<EnvState, String> {
    // Node.js
    let node_version = get_version("node", &["--version"]).await;
    let node_path = find_path("node").await;
    let node = NodeInfo {
        installed: node_version.is_some(),
        version: node_version.as_ref().and_then(|v| parse_node_version(v)),
        path: node_path,
        meets_minimum: node_version
            .as_ref()
            .and_then(|v| parse_node_version(v))
            .map(|v| check_node_meets_minimum(&v)),
    };

    // npm
    let npm_version = get_version("npm", &["--version"]).await;
    let npm_path = find_path("npm").await;
    let npm = NpmInfo {
        installed: npm_version.is_some(),
        version: npm_version.map(|v| v.trim().to_string()),
        path: npm_path,
    };

    // dsh - via npx to avoid PATH issues
    let dsh_version_raw = get_version("npx", &["@deepseek-ai/dsh", "-V"]).await;
    let dsh_path = find_path("npx").await;
    let dsh_version = dsh_version_raw.as_ref().and_then(|v| parse_dsh_version(v));

    // Check latest dsh version from npm registry
    let latest_version = match reqwest::Client::new()
        .get("https://registry.npmjs.org/@deepseek-ai/dsh/latest")
        .send()
        .await
    {
        Ok(resp) => {
            if let Ok(json) = resp.json::<serde_json::Value>().await {
                json["version"].as_str().map(|s| s.to_string())
            } else {
                None
            }
        }
        Err(_) => None,
    };

    let update_available = match (&dsh_version, &latest_version) {
        (Some(cur), Some(latest)) => {
            // Compare semver versions
            match (Version::parse(cur), Version::parse(latest)) {
                (Ok(c), Ok(l)) => c < l,
                _ => cur != latest,
            }
        }
        _ => false,
    };

    let dsh = DshInfo {
        installed: dsh_version.is_some(),
        version: dsh_version,
        path: dsh_path,
        latest_version,
        update_available,
    };

    // DSH_HOME
    let dsh_home_path = get_dsh_home();
    let dsh_home = DshHomeInfo {
        exists: dsh_home_path.exists(),
        path: dsh_home_path.to_string_lossy().to_string(),
        profiles_dir: dsh_home_path.join("profiles").exists(),
        sessions_dir: dsh_home_path.join("sessions").exists(),
    };

    Ok(EnvState {
        node,
        npm,
        dsh,
        dsh_home,
    })
}

#[tauri::command]
pub async fn check_node_version() -> Result<NodeInfo, String> {
    let node_version = get_version("node", &["--version"]).await;
    let node_path = find_path("node").await;
    Ok(NodeInfo {
        installed: node_version.is_some(),
        version: node_version.as_ref().and_then(|v| parse_node_version(v)),
        path: node_path,
        meets_minimum: node_version
            .as_ref()
            .and_then(|v| parse_node_version(v))
            .map(|v| check_node_meets_minimum(&v)),
    })
}

#[tauri::command]
pub async fn check_dsh_version() -> Result<DshInfo, String> {
    let dsh_version_raw = get_version("npx", &["@deepseek-ai/dsh", "-V"]).await;
    let dsh_path = find_path("npx").await;
    let dsh_version = dsh_version_raw.as_ref().and_then(|v| parse_dsh_version(v));

    Ok(DshInfo {
        installed: dsh_version.is_some(),
        version: dsh_version,
        path: dsh_path,
        latest_version: None,
        update_available: false,
    })
}
