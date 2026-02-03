use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const FORMULA_URL: &str =
    "https://raw.githubusercontent.com/ZacharyAnderson/homebrew-scrap/main/Formula/scrap.rb";
const CACHE_DURATION_SECS: u64 = 24 * 60 * 60; // 24 hours
const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Serialize, Deserialize)]
struct VersionCache {
    latest_version: String,
    checked_at: u64,
    notified_version: Option<String>,
}

fn cache_path() -> Result<PathBuf> {
    let home = dirs::home_dir().ok_or_else(|| anyhow::anyhow!("No home directory"))?;
    Ok(home.join(".scrap").join("version_cache.json"))
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::ZERO)
        .as_secs()
}

fn read_cache() -> Option<VersionCache> {
    let path = cache_path().ok()?;
    let content = fs::read_to_string(path).ok()?;
    serde_json::from_str(&content).ok()
}

fn write_cache(cache: &VersionCache) {
    if let Ok(path) = cache_path() {
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let _ = fs::write(path, serde_json::to_string(cache).unwrap_or_default());
    }
}

fn fetch_latest_version() -> Result<String> {
    let response = reqwest::blocking::Client::new()
        .get(FORMULA_URL)
        .timeout(Duration::from_secs(5))
        .send()?
        .text()?;

    // Parse version from formula - look for tag: "v0.1.1" pattern anywhere in line
    for line in response.lines() {
        if let Some(tag_pos) = line.find("tag:") {
            let after_tag = &line[tag_pos..];
            if let Some(start) = after_tag.find('"') {
                if let Some(end) = after_tag[start + 1..].find('"') {
                    let version = &after_tag[start + 1..start + 1 + end];
                    return Ok(version.trim_start_matches('v').to_string());
                }
            }
        }
    }

    anyhow::bail!("Could not parse version from formula")
}

fn version_is_newer(latest: &str, current: &str) -> bool {
    let parse = |v: &str| -> Vec<u32> {
        v.split('.')
            .filter_map(|p| p.parse().ok())
            .collect()
    };
    let latest_parts = parse(latest);
    let current_parts = parse(current);

    for i in 0..latest_parts.len().max(current_parts.len()) {
        let l = latest_parts.get(i).copied().unwrap_or(0);
        let c = current_parts.get(i).copied().unwrap_or(0);
        if l > c {
            return true;
        }
        if l < c {
            return false;
        }
    }
    false
}

/// Check for updates and print a message if a new version is available.
/// This is designed to be non-disruptive - failures are silently ignored.
pub fn check_for_updates() {
    let result = check_for_updates_inner();
    if let Some(latest) = result {
        eprintln!(
            "\x1b[33m╭─────────────────────────────────────────────────────╮\x1b[0m"
        );
        eprintln!(
            "\x1b[33m│\x1b[0m  A new version of scrap is available: \x1b[32m{}\x1b[0m (you have {})  \x1b[33m│\x1b[0m",
            latest, CURRENT_VERSION
        );
        eprintln!(
            "\x1b[33m│\x1b[0m  Run \x1b[36mbrew upgrade scrap\x1b[0m to update.                   \x1b[33m│\x1b[0m"
        );
        eprintln!(
            "\x1b[33m╰─────────────────────────────────────────────────────╯\x1b[0m"
        );
        eprintln!();
    }
}

fn check_for_updates_inner() -> Option<String> {
    let now = now_secs();

    // Check cache first
    if let Some(cache) = read_cache() {
        let age = now.saturating_sub(cache.checked_at);

        if age < CACHE_DURATION_SECS {
            // Cache is fresh - check if we should notify
            if version_is_newer(&cache.latest_version, CURRENT_VERSION) {
                // Only notify once per version
                if cache.notified_version.as_deref() != Some(&cache.latest_version) {
                    // Update cache to mark as notified
                    let version = cache.latest_version.clone();
                    let updated_cache = VersionCache {
                        notified_version: Some(version.clone()),
                        latest_version: version.clone(),
                        checked_at: cache.checked_at,
                    };
                    write_cache(&updated_cache);
                    return Some(version);
                }
            }
            return None;
        }
    }

    // Cache is stale or missing - fetch fresh
    let latest = fetch_latest_version().ok()?;

    let should_notify = version_is_newer(&latest, CURRENT_VERSION);

    let cache = VersionCache {
        latest_version: latest.clone(),
        checked_at: now,
        notified_version: if should_notify { Some(latest.clone()) } else { None },
    };
    write_cache(&cache);

    if should_notify {
        Some(latest)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_comparison() {
        assert!(version_is_newer("0.2.0", "0.1.0"));
        assert!(version_is_newer("0.1.1", "0.1.0"));
        assert!(version_is_newer("1.0.0", "0.9.9"));
        assert!(!version_is_newer("0.1.0", "0.1.0"));
        assert!(!version_is_newer("0.1.0", "0.2.0"));
    }
}
