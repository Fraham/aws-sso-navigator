use dirs::home_dir;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Serialize, Deserialize, Default)]
pub struct Settings {
    pub default_client: Option<String>,
    pub default_account: Option<String>,
    pub default_role: Option<String>,
    pub unified_mode: Option<bool>,
    pub set_default: Option<bool>,
    pub list: Option<bool>,
    pub recent: Option<bool>,
    pub max_recent_profiles: Option<usize>,
    pub aws_config_path: Option<String>,
}

#[derive(Serialize, Deserialize, Default)]
pub struct RecentProfiles {
    pub profiles: HashMap<String, u64>,
}

pub fn load_settings() -> Settings {
    let settings_path = home_dir()
        .unwrap()
        .join(".config")
        .join("aws-sso-navigator")
        .join("config.toml");

    if settings_path.exists() {
        let contents = fs::read_to_string(&settings_path).unwrap_or_default();
        toml::from_str(&contents).unwrap_or_default()
    } else {
        Settings::default()
    }
}

pub fn load_recent_profiles() -> RecentProfiles {
    let config_dir = home_dir()
        .unwrap()
        .join(".config")
        .join("aws-sso-navigator");
    
    let recent_path = config_dir.join("recent.toml");
    if recent_path.exists() {
        let contents = fs::read_to_string(&recent_path).unwrap_or_default();
        toml::from_str(&contents).unwrap_or_default()
    } else {
        RecentProfiles::default()
    }
}

pub fn save_recent_profile(profile_name: &str, max_entries: usize) {
    let config_dir = home_dir()
        .unwrap()
        .join(".config")
        .join("aws-sso-navigator");
    
    fs::create_dir_all(&config_dir).ok();
    
    let mut recent = load_recent_profiles();
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    recent.profiles.insert(profile_name.to_string(), timestamp);

    // Keep only the configured number of most recent profiles
    if recent.profiles.len() > max_entries {
        let mut sorted: Vec<_> = recent.profiles.iter().collect();
        sorted.sort_by(|a, b| b.1.cmp(a.1)); // Sort by timestamp descending
        recent.profiles = sorted.into_iter().take(max_entries).map(|(k, v)| (k.clone(), *v)).collect();
    }

    let recent_path = config_dir.join("recent.toml");
    if let Ok(contents) = toml::to_string(&recent) {
        fs::write(&recent_path, contents).ok();
    }
}