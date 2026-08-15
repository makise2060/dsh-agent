use crate::cmd_ext::hidden_command;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter};
use tokio::io::{AsyncBufReadExt, BufReader};

use crate::env::get_dsh_home;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PluginRepo {
    pub full_name: String,
    pub description: Option<String>,
    pub html_url: String,
    pub stargazers_count: u32,
    pub topics: Vec<String>,
    pub updated_at: String,
    pub owner_avatar: String,
    pub license: Option<String>,
    pub installed: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PluginSearchResult {
    pub repos: Vec<PluginRepo>,
    pub total_count: u32,
    pub page: u32,
    pub has_more: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct InstalledPlugin {
    pub name: String,
    pub version: String,
    pub path: Option<String>,
}

#[derive(serde::Serialize, Clone)]
struct PluginInstallProgress {
    package: String,
    stage: String,
    message: String,
}

#[tauri::command]
pub async fn search_plugins(
    query: Option<String>,
    sort: Option<String>,
    page: Option<u32>,
) -> Result<PluginSearchResult, String> {
    let page = page.unwrap_or(1);
    let per_page = 30;

    let sort_param = match sort.as_deref() {
        Some("updated") => "updated",
        Some("name") => "name",
        _ => "stars",
    };

    let mut url = format!(
        "https://api.github.com/search/repositories?q=topic:dsh-plugin&sort={}&order=desc&per_page={}&page={}",
        sort_param, per_page, page
    );

    if let Some(q) = query {
        if !q.is_empty() {
            url = format!(
                "https://api.github.com/search/repositories?q=topic:dsh-plugin+{}&sort={}&order=desc&per_page={}&page={}",
                q, sort_param, per_page, page
            );
        }
    }

    let resp: serde_json::Value = reqwest::Client::new()
        .get(&url)
        .header("Accept", "application/vnd.github+json")
        .header("User-Agent", "dsh-agent")
        .send()
        .await
        .map_err(|e| format!("GitHub API error: {}", e))?
        .json()
        .await
        .map_err(|e| format!("Parse error: {}", e))?;

    let total_count = resp["total_count"]
        .as_u64()
        .unwrap_or(0) as u32;

    let items = resp["items"]
        .as_array()
        .cloned()
        .unwrap_or_default();

    // Get installed packages for cross-referencing
    let installed_names = get_installed_package_names().await.unwrap_or_default();

    let repos: Vec<PluginRepo> = items
        .iter()
        .map(|item| {
            let full_name = item["full_name"].as_str().unwrap_or("").to_string();
            let installed = installed_names
                .iter()
                .any(|name| full_name.contains(name));
            PluginRepo {
                full_name,
                description: item["description"].as_str().map(|s| s.to_string()),
                html_url: item["html_url"].as_str().unwrap_or("").to_string(),
                stargazers_count: item["stargazers_count"]
                    .as_u64()
                    .unwrap_or(0) as u32,
                topics: item["topics"]
                    .as_array()
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|t| t.as_str().map(|s| s.to_string()))
                            .collect()
                    })
                    .unwrap_or_default(),
                updated_at: item["updated_at"]
                    .as_str()
                    .unwrap_or("")
                    .to_string(),
                owner_avatar: item["owner"]["avatar_url"]
                    .as_str()
                    .unwrap_or("")
                    .to_string(),
                license: item["license"]["spdx_id"]
                    .as_str()
                    .map(|s| s.to_string()),
                installed,
            }
        })
        .collect();

    let has_more = (page * per_page) < total_count;

    Ok(PluginSearchResult {
        repos,
        total_count,
        page,
        has_more,
    })
}

#[tauri::command]
pub async fn list_installed_plugins() -> Result<Vec<InstalledPlugin>, String> {
    let names = get_installed_package_names().await.unwrap_or_default();
    let home = get_dsh_home();
    let node_modules = home.join("profiles").join("web").join("node_modules");

    let plugins: Vec<InstalledPlugin> = names
        .iter()
        .map(|name| InstalledPlugin {
            name: name.clone(),
            version: "unknown".to_string(),
            path: node_modules.join(name).to_str().map(|s| s.to_string()),
        })
        .collect();

    Ok(plugins)
}

#[tauri::command]
pub async fn install_plugin(app: AppHandle, package_name: String) -> Result<(), String> {
    let pkg = package_name.clone();
    let _ = app.emit(
        "plugin-install-progress",
        PluginInstallProgress {
            package: pkg.clone(),
            stage: "starting".to_string(),
            message: format!("正在安装 {}...", pkg),
        },
    );

    #[cfg(target_os = "windows")]
    let mut child = {
        let npx = crate::cmd_ext::resolve_global_bin("npx").await;
        hidden_command("cmd.exe")
            .args(["/C", npx.as_str(), "@deepseek-ai/dsh", "plugin", "--profile", "web", "add", &pkg])
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| format!("Failed to run dsh plugin add: {}", e))?
    };

    #[cfg(not(target_os = "windows"))]
    let mut child = hidden_command("npx")
        .args(["@deepseek-ai/dsh", "plugin", "--profile", "web", "add", &pkg])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to run dsh plugin add: {}", e))?;

    // Stream output
    if let Some(stdout) = child.stdout.take() {
        let reader = BufReader::new(stdout);
        let mut lines = reader.lines();
        while let Ok(Some(line)) = lines.next_line().await {
            let _ = app.emit(
                "plugin-install-progress",
                PluginInstallProgress {
                    package: pkg.clone(),
                    stage: "progress".to_string(),
                    message: line,
                },
            );
        }
    }

    if let Some(stderr) = child.stderr.take() {
        let reader = BufReader::new(stderr);
        let mut lines = reader.lines();
        while let Ok(Some(line)) = lines.next_line().await {
            let _ = app.emit(
                "plugin-install-progress",
                PluginInstallProgress {
                    package: pkg.clone(),
                    stage: "progress".to_string(),
                    message: line,
                },
            );
        }
    }

    let status = child
        .wait()
        .await
        .map_err(|e| format!("Failed to wait: {}", e))?;

    if status.success() {
        let _ = app.emit(
            "plugin-install-progress",
            PluginInstallProgress {
                package: pkg,
                stage: "done".to_string(),
                message: "安装成功".to_string(),
            },
        );
        Ok(())
    } else {
        let _ = app.emit(
            "plugin-install-progress",
            PluginInstallProgress {
                package: pkg,
                stage: "error".to_string(),
                message: format!("安装失败 (exit code: {})", status.code().unwrap_or(-1)),
            },
        );
        Err(format!("Plugin install failed"))
    }
}

#[tauri::command]
pub async fn remove_plugin(app: AppHandle, package_name: String) -> Result<(), String> {
    let pkg = package_name.clone();
    let _ = app.emit(
        "plugin-install-progress",
        PluginInstallProgress {
            package: pkg.clone(),
            stage: "starting".to_string(),
            message: format!("正在移除 {}...", pkg),
        },
    );

    #[cfg(target_os = "windows")]
    let output = {
        let npx = crate::cmd_ext::resolve_global_bin("npx").await;
        hidden_command("cmd.exe")
            .args(["/C", npx.as_str(), "@deepseek-ai/dsh", "plugin", "--profile", "web", "remove", &pkg])
            .output()
            .await
            .map_err(|e| format!("Failed to run dsh plugin remove: {}", e))?
    };

    #[cfg(not(target_os = "windows"))]
    let output = hidden_command("npx")
        .args(["@deepseek-ai/dsh", "plugin", "--profile", "web", "remove", &pkg])
        .output()
        .await
        .map_err(|e| format!("Failed to run dsh plugin remove: {}", e))?;

    if output.status.success() {
        let _ = app.emit(
            "plugin-install-progress",
            PluginInstallProgress {
                package: pkg,
                stage: "done".to_string(),
                message: "移除成功".to_string(),
            },
        );
        Ok(())
    } else {
        Err("Plugin remove failed".to_string())
    }
}

#[tauri::command]
pub async fn activate_plugin(plugin_id: String, plugin_name: String) -> Result<(), String> {
    let home = get_dsh_home();
    let patch_path = home.join("profiles").join("web").join("cordis.patch.yml");

    // Read current content
    let content = std::fs::read_to_string(&patch_path)
        .unwrap_or_else(|_| "[]\n".to_string());

    // Parse as YAML array
    let mut entries: Vec<serde_json::Value> = serde_yaml::from_str(&content)
        .unwrap_or_default();

    // Check if plugin already activated
    let already = entries.iter().any(|e| {
        e["name"].as_str() == Some(&plugin_name)
    });

    if already {
        return Ok(());
    }

    // Add new entry
    let new_entry = serde_json::json!({
        "id": plugin_id,
        "name": plugin_name,
    });
    entries.push(new_entry);

    // Write back
    let yaml = serde_yaml::to_string(&entries).map_err(|e| e.to_string())?;
    std::fs::write(&patch_path, yaml).map_err(|e| e.to_string())?;

    Ok(())
}

/// Get installed package names from the web profile's package.json dependencies
async fn get_installed_package_names() -> Result<Vec<String>, String> {
    let home = get_dsh_home();
    let pkg_json_path = home.join("profiles").join("web").join("package.json");

    if !pkg_json_path.exists() {
        return Ok(vec![]);
    }

    let content = std::fs::read_to_string(&pkg_json_path).map_err(|e| e.to_string())?;
    let pkg: serde_json::Value = serde_json::from_str(&content).map_err(|e| e.to_string())?;

    let deps = pkg["dependencies"].as_object();
    let names: Vec<String> = deps
        .map(|d| d.keys().cloned().collect())
        .unwrap_or_default();

    Ok(names)
}
