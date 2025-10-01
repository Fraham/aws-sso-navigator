use clap::Parser;
use dirs::home_dir;
use regex::Regex;
use serde::{Deserialize, Serialize};
use skim::prelude::*;
use std::collections::BTreeSet;
use std::fs;
use std::io::Cursor;
use std::path::PathBuf;
use std::process::Command;

#[derive(Debug, Clone)]
struct Profile {
    client: String,
    account: String,
    role: String,
    name: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct Settings {
    default_client: Option<String>,
    default_account: Option<String>,
    default_role: Option<String>,
    unified_mode: Option<bool>,
    aws_config_path: Option<PathBuf>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            default_client: None,
            default_account: None,
            default_role: None,
            unified_mode: Some(false),
            aws_config_path: None,
        }
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
}

fn load_profiles(config_path: PathBuf) -> Vec<Profile> {
    let contents = fs::read_to_string(config_path).expect("Failed to read AWS config");
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

fn skim_pick(prompt: &str, options: Vec<String>) -> Option<String> {
    let input = options.join("\n");
    let prompt_str = format!("{}> ", prompt);
    let options = SkimOptionsBuilder::default()
        .prompt(Some(&prompt_str))
        .height(Some("50%"))
        .multi(false)
        .bind(vec!["esc:abort"])
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
    let profiles = load_profiles(config_path);
    if profiles.is_empty() {
        eprintln!("No profiles found");
        std::process::exit(1);
    }
    let settings = load_settings();
    
    let mut chosen_client = args.client.or(settings.default_client);
    let mut chosen_account = args.account.or(settings.default_account);
    let mut chosen_role = args.role.or(settings.default_role);
    let unified_mode = args.unified || settings.unified_mode.unwrap_or(false);
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
            let clients: Vec<String> = profiles.iter().map(|p| p.client.clone()).collect::<BTreeSet<_>>().into_iter().collect();
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
        if let (Some(client), Some(account), None) = (&chosen_client, &chosen_account, &chosen_role) {
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
            
            if args.set_default {
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
