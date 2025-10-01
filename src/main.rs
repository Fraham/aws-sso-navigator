use clap::Parser;
use dirs::home_dir;
use regex::Regex;
use serde::{Deserialize, Serialize};
use skim::prelude::*;
use std::collections::{BTreeSet, HashMap};
use std::fs;
use std::io::Cursor;
use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone)]
struct Profile {
    client: String,
    account: String,
    role: String,
    name: String,
}

fn load_profiles(config_path: &PathBuf) -> Vec<Profile> {
    let contents = fs::read_to_string(config_path).unwrap_or_default();
    let re = Regex::new(r"^\[profile (.+)\]").unwrap();
    let mut profiles = Vec::new();
    for line in contents.lines() {
        if let Some(cap) = re.captures(line) {
            let profile_name = cap[1].to_string();
            let parts: Vec<&str> = profile_name.split('-').collect();
            if parts.len() >= 3 {
                let client = parts[0].to_string();
                let account = parts[1].to_string();
                let role = parts[2..].join("-");
                profiles.push(Profile {
                    client,
                    account,
                    role,
                    name: profile_name,
                });
            }
        }
    }
    profiles
}

#[derive(Debug, Deserialize, Serialize)]
struct Settings {
    default_client: Option<String>,
    default_account: Option<String>,
    default_role: Option<String>,
    unified_mode: Option<bool>,
    aws_config_path: Option<PathBuf>,
    set_default: Option<bool>,
    list: Option<bool>,
    recent: Option<bool>,
    max_recent_profiles: Option<usize>,
}

#[derive(Debug, Deserialize, Serialize, Default)]
struct RecentProfiles {
    profiles: HashMap<String, u64>, // profile_name -> timestamp
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            default_client: None,
            default_account: None,
            default_role: None,
            unified_mode: Some(false),
            aws_config_path: None,
            set_default: Some(false),
            list: Some(false),
            recent: Some(false),
            max_recent_profiles: Some(100),
        }
    }
}

fn load_recent_profiles() -> RecentProfiles {
    let recent_path = home_dir()
        .unwrap()
        .join(".config")
        .join("aws-sso-navigator")
        .join("recent.toml");

    if recent_path.exists() {
        let contents = fs::read_to_string(&recent_path).unwrap_or_default();
        toml::from_str(&contents).unwrap_or_default()
    } else {
        RecentProfiles::default()
    }
}

fn save_recent_profile(profile_name: &str, max_entries: usize) {
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

fn load_settings() -> Settings {
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

fn set_default_profile(profile_name: &str) -> Result<(), std::io::Error> {
    unsafe {
        std::env::set_var("AWS_PROFILE", profile_name);
    }
    println!("Set {} as default AWS profile", profile_name);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_load_profiles_empty_file() {
        let temp_file = NamedTempFile::new().unwrap();
        let profiles = load_profiles(&temp_file.path().to_path_buf());
        assert!(profiles.is_empty());
    }

    #[test]
    fn test_load_profiles_valid_format() {
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "[profile client1-dev-admin]").unwrap();
        writeln!(temp_file, "sso_start_url = https://example.com").unwrap();
        writeln!(temp_file, "[profile client2-prod-readonly]").unwrap();

        let profiles = load_profiles(&temp_file.path().to_path_buf());
        assert_eq!(profiles.len(), 2);

        assert_eq!(profiles[0].client, "client1");
        assert_eq!(profiles[0].account, "dev");
        assert_eq!(profiles[0].role, "admin");
        assert_eq!(profiles[0].name, "client1-dev-admin");

        assert_eq!(profiles[1].client, "client2");
        assert_eq!(profiles[1].account, "prod");
        assert_eq!(profiles[1].role, "readonly");
    }

    #[test]
    fn test_load_profiles_invalid_format() {
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "[profile invalid]").unwrap();
        writeln!(temp_file, "[profile client-dev]").unwrap();
        writeln!(temp_file, "[profile valid-dev-admin]").unwrap();

        let profiles = load_profiles(&temp_file.path().to_path_buf());
        assert_eq!(profiles.len(), 1);
        assert_eq!(profiles[0].name, "valid-dev-admin");
    }

    #[test]
    fn test_load_profiles_multi_part_role() {
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "[profile client-dev-power-user-access]").unwrap();

        let profiles = load_profiles(&temp_file.path().to_path_buf());
        assert_eq!(profiles.len(), 1);
        assert_eq!(profiles[0].role, "power-user-access");
    }
}

#[derive(Parser, Debug)]
#[command(
    name = "aws-sso-navigator",
    version = "0.1.0",
    about = "Navigate and login to AWS SSO profiles with fuzzy selection",
    long_about = "A CLI tool to interactively select and login to AWS SSO profiles. Supports both step-by-step and unified selection modes."
)]
struct Args {
    /// Optional client to skip selection
    #[arg(long)]
    client: Option<String>,
    /// Optional account to skip selection
    #[arg(long)]
    account: Option<String>,
    /// Optional role to skip selection
    #[arg(long)]
    role: Option<String>,
    /// If set, use a unified picker instead of step-by-step
    #[arg(long)]
    unified: bool,
    /// Path to AWS config
    #[arg(long)]
    aws_config_path: Option<PathBuf>,
    /// Set the selected profile as the default AWS profile
    #[arg(long)]
    set_default: bool,
    /// List all profiles without selection
    #[arg(long)]
    list: bool,
    /// Show recently used profiles first
    #[arg(long)]
    recent: bool,
}

fn skim_pick(prompt: &str, options: Vec<String>) -> Option<String> {
    let input = options.join("\n");
    let prompt_str = format!("{}> ", prompt);
    let options = SkimOptionsBuilder::default()
        .prompt(prompt_str)
        .height("50%".to_string())
        .multi(false)
        .bind(vec!["esc:abort".to_string()])
        .build()
        .unwrap();
    let item_reader = SkimItemReader::default();
    let items = item_reader.of_bufread(Cursor::new(input));
    let output = Skim::run_with(&options, Some(items))?;

    // Clear the screen after skim
    print!("\x1B[2J\x1B[1;1H");

    if output.is_abort {
        return None;
    }
    if output.selected_items.is_empty() {
        None
    } else {
        Some(output.selected_items[0].output().to_string())
    }
}

fn main() {
    let args = Args::parse();
    let config_path = args
        .aws_config_path
        .unwrap_or_else(|| home_dir().unwrap().join(".aws").join("config"));
    let mut profiles = load_profiles(&config_path);

    if profiles.is_empty() {
        eprintln!("No profiles found");
        std::process::exit(1);
    }
    let settings = load_settings();

    let mut chosen_client = args.client.or(settings.default_client);
    let mut chosen_account = args.account.or(settings.default_account);
    let mut chosen_role = args.role.or(settings.default_role);
    let unified_mode = args.unified || settings.unified_mode.unwrap_or(false);
    let set_default = args.set_default || settings.set_default.unwrap_or(false);
    let list = args.list || settings.list.unwrap_or(false);
    let recent = args.recent || settings.recent.unwrap_or(false);

    if recent {
        let recent = load_recent_profiles();
        profiles.sort_by(|a, b| {
            let a_time = recent.profiles.get(&a.name).unwrap_or(&0);
            let b_time = recent.profiles.get(&b.name).unwrap_or(&0);
            b_time.cmp(a_time)
        });
    }

    if list {
        for profile in &profiles {
            println!("{}", profile.name);
        }
        return;
    }

    if unified_mode {
        // Unified picker mode
        let rows: Vec<String> = profiles
            .iter()
            .map(|p| format!("{} | {} | {} | {}", p.client, p.account, p.role, p.name))
            .collect();
        match skim_pick("Select Profile", rows) {
            Some(choice) => {
                let parts: Vec<&str> = choice.split('|').map(|s| s.trim()).collect();
                chosen_client = Some(parts[0].to_string());
                chosen_account = Some(parts[1].to_string());
                chosen_role = Some(parts[2].to_string());
            }
            None => return,
        }
    } else {
        // Step-by-step mode
        if chosen_client.is_none() {
            let clients: Vec<String> = profiles
                .iter()
                .map(|p| p.client.clone())
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect();
            chosen_client = match skim_pick("Select Client", clients) {
                Some(client) => Some(client),
                None => return,
            };
        }
        if let (Some(client), None) = (&chosen_client, &chosen_account) {
            let accounts: Vec<String> = profiles
                .iter()
                .filter(|p| &p.client == client)
                .map(|p| p.account.clone())
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect();
            chosen_account = match skim_pick("Select Account", accounts) {
                Some(account) => Some(account),
                None => return,
            };
        }
        if let (Some(client), Some(account), None) = (&chosen_client, &chosen_account, &chosen_role)
        {
            let roles: Vec<String> = profiles
                .iter()
                .filter(|p| &p.client == client && &p.account == account)
                .map(|p| p.role.clone())
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect();
            chosen_role = match skim_pick("Select Role", roles) {
                Some(role) => Some(role),
                None => return,
            };
        }
    }
    // Resolve final profile
    if let (Some(client), Some(account), Some(role)) = (chosen_client, chosen_account, chosen_role)
    {
        if let Some(profile) = profiles
            .iter()
            .find(|p| p.client == client && p.account == account && p.role == role)
        {
            println!("Logging into AWS profile: {}", profile.name);
            let status = Command::new("aws")
                .arg("sso")
                .arg("login")
                .arg("--profile")
                .arg(&profile.name)
                .status()
                .expect("Failed to execute aws");
            if !status.success() {
                eprintln!("AWS SSO login failed");
                std::process::exit(1);
            }

            let max_recent = settings.max_recent_profiles.unwrap_or(100);
            save_recent_profile(&profile.name, max_recent);

            if set_default {
                if let Err(e) = set_default_profile(&profile.name) {
                    eprintln!("Warning: Failed to set default profile: {}", e);
                }
            }
        } else {
            eprintln!("No matching profile found");
            std::process::exit(1);
        }
    } else {
        eprintln!("Selection incomplete");
        std::process::exit(1);
    }
}
