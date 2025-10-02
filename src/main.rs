mod aws;
mod config;
mod profile;
mod ui;

use clap::Parser;
use dirs::home_dir;
use std::path::PathBuf;

use config::{load_recent_profiles, load_settings, save_recent_profile};
use profile::{load_profiles, select_unique_values};
use ui::skim_pick;

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
    /// If set, use step-by-step mode (overrides config unified_mode)
    #[arg(long)]
    step_by_step: bool,
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

    let unified_mode = if args.step_by_step {
        false
    } else {
        args.unified || settings.unified_mode.unwrap_or_default()
    };
    let set_default = args.set_default || settings.set_default.unwrap_or_default();
    let list = args.list || settings.list.unwrap_or_default();
    let recent = args.recent || settings.recent.unwrap_or_default();

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
        let rows: Vec<String> = profiles
            .iter()
            .map(|p| format!("{} | {} | {} | {}", p.client, p.account, p.role, p.name))
            .collect();
        let Some(choice) = skim_pick("Select Profile", rows) else { return };
        let parts: Vec<&str> = choice.split('|').map(|s| s.trim()).collect();
        chosen_client = Some(parts[0].to_string());
        chosen_account = Some(parts[1].to_string());
        chosen_role = Some(parts[2].to_string());
    } else {
        if chosen_client.is_none() {
            let Some(client) = select_unique_values(&profiles, |p| p.client.clone(), "Select Client") else { return };
            chosen_client = Some(client);
        }
        if let (Some(client), None) = (&chosen_client, &chosen_account) {
            let filtered: Vec<_> = profiles.iter().filter(|p| &p.client == client).cloned().collect();
            let Some(account) = select_unique_values(&filtered, |p| p.account.clone(), "Select Account") else { return };
            chosen_account = Some(account);
        }
        if let (Some(client), Some(account), None) = (&chosen_client, &chosen_account, &chosen_role) {
            let filtered: Vec<_> = profiles.iter().filter(|p| &p.client == client && &p.account == account).cloned().collect();
            let Some(role) = select_unique_values(&filtered, |p| p.role.clone(), "Select Role") else { return };
            chosen_role = Some(role);
        }
    }

    let (Some(client), Some(account), Some(role)) = (chosen_client, chosen_account, chosen_role) else {
        eprintln!("Selection incomplete");
        std::process::exit(1);
    };

    let Some(profile) = profiles.iter().find(|p| p.client == client && p.account == account && p.role == role) else {
        eprintln!("No matching profile found");
        std::process::exit(1);
    };

    if let Err(e) = aws::login_to_profile(&profile.name) {
        eprintln!("{}", e);
        std::process::exit(1);
    }

    let max_recent = settings.max_recent_profiles.unwrap_or(100);
    save_recent_profile(&profile.name, max_recent);

    if set_default {
        aws::set_default_profile(&profile.name);
    }
}