use serde::{Deserialize, Serialize};
use semver::Version;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct UpdateInfo {
    pub current_version: String,
    pub latest_version: String,
    pub update_available: bool,
    pub release_notes: Option<String>,
}

#[tauri::command]
pub async fn check_dsh_update() -> Result<UpdateInfo, String> {
    // Get current installed version via npx
    #[cfg(target_os = "windows")]
    let current_output = {
        let npx = crate::cmd_ext::resolve_global_bin("npx").await;
        crate::cmd_ext::silent_command("cmd.exe")
            .args(["/C", npx.as_str(), "@deepseek-ai/dsh", "-V"])
            .output()
            .await
            .map_err(|e| format!("Failed to run dsh -V: {}", e))?
    };

    #[cfg(not(target_os = "windows"))]
    let current_output = crate::cmd_ext::silent_command("npx")
        .args(["@deepseek-ai/dsh", "-V"])
        .output()
        .await
        .map_err(|e| format!("Failed to run dsh -V: {}", e))?;

    let current_raw = String::from_utf8_lossy(&current_output.stdout).trim().to_string();
    let current = parse_version(&current_raw).unwrap_or(current_raw);

    // Get latest from npm registry
    let resp: serde_json::Value = reqwest::Client::new()
        .get("https://registry.npmjs.org/@deepseek-ai/dsh/latest")
        .send()
        .await
        .map_err(|e| format!("Network error: {}", e))?
        .json()
        .await
        .map_err(|e| format!("Parse error: {}", e))?;

    let latest = resp["version"]
        .as_str()
        .ok_or("Missing version in npm response")?
        .to_string();

    let update_available = compare_versions(&current, &latest);

    Ok(UpdateInfo {
        current_version: current.clone(),
        latest_version: latest,
        update_available,
        release_notes: None,
    })
}

#[tauri::command]
pub async fn check_app_update() -> Result<UpdateInfo, String> {
    let current = env!("CARGO_PKG_VERSION").to_string();

    // Query the latest GitHub release and compare with the running version.
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| format!("Failed to build client: {}", e))?;

    let resp = client
        .get("https://api.github.com/repos/makise2060/dsh-agent/releases/latest")
        // GitHub API rejects requests without a User-Agent
        .header("User-Agent", "dsh-agent")
        .send()
        .await
        .map_err(|e| format!("Network error: {}", e))?;

    if !resp.status().is_success() {
        return Err(format!("GitHub API error: HTTP {}", resp.status()));
    }

    let json: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("Parse error: {}", e))?;

    let latest = json["tag_name"]
        .as_str()
        .unwrap_or_default()
        .trim_start_matches('v')
        .to_string();
    if latest.is_empty() {
        return Err("Missing tag_name in GitHub response".to_string());
    }

    let release_notes = json["body"].as_str().map(|s| s.to_string());
    let update_available = compare_versions(&current, &latest);

    Ok(UpdateInfo {
        current_version: current,
        latest_version: latest,
        update_available,
        release_notes,
    })
}

fn parse_version(raw: &str) -> Option<String> {
    let v = raw.trim();
    match Version::parse(v) {
        Ok(_) => Some(v.to_string()),
        Err(_) => {
            for part in v.split_whitespace() {
                if Version::parse(part).is_ok() {
                    return Some(part.to_string());
                }
            }
            None
        }
    }
}

fn compare_versions(current: &str, latest: &str) -> bool {
    match (Version::parse(current), Version::parse(latest)) {
        (Ok(c), Ok(l)) => c < l,
        _ => current != latest,
    }
}
