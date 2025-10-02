mod aws;
mod config;
mod profile;
mod ui;
mod import;

use clap::Parser;
use dirs::home_dir;
use std::path::PathBuf;

use config::{load_recent_profiles, load_settings, save_recent_profile};
use profile::{load_profiles, select_filtered_values, select_unique_values};
use ui::skim_pick;

#[derive(Parser, Debug)]
#[command(
    name = "aws-sso-navigator",
    version = "0.1.0",
    about = "Navigate and login to AWS SSO profiles with fuzzy selection",
    long_about = "A CLI tool to interactively select and login to AWS SSO profiles. Supports both step-by-step and unified selection modes."
)]
struct Args {
    #[command(subcommand)]
    command: Option<Commands>,
    /// Path to AWS config
    #[arg(long, global = true)]
    aws_config_path: Option<PathBuf>,
}

#[derive(Parser, Debug)]
enum Commands {
    /// Authenticate to AWS SSO profiles (default)
    Auth(AuthArgs),
    /// Import profiles from SSO session
    Import(ImportArgs),
}

#[derive(Parser, Debug)]
struct AuthArgs {
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
    /// Set the selected profile as the default AWS profile
    #[arg(long)]
    set_default: bool,
    /// List all profiles without selection
    #[arg(long)]
    list: bool,
    /// Show recently used profiles first
    #[arg(long)]
    recent: bool,
    /// Force reauthentication even if session is valid
    #[arg(long)]
    force_reauth: bool,
    /// Open AWS console in browser instead of logging in via CLI
    #[arg(long)]
    console: bool,
}

#[derive(Parser, Debug)]
struct ImportArgs {
    /// SSO session name to import profiles from
    sso_session: String,
}

fn main() {
    let args = Args::parse();
    let config_path = args
        .aws_config_path
        .unwrap_or_else(|| home_dir().unwrap().join(".aws").join("config"));
    
    match args.command.unwrap_or(Commands::Auth(AuthArgs {
        client: None,
        account: None,
        role: None,
        unified: false,
        step_by_step: false,
        set_default: false,
        list: false,
        recent: false,
        force_reauth: false,
        console: false,
    })) {
        Commands::Import(import_args) => {
            if let Err(e) = import::import_profiles(&import_args.sso_session, &config_path) {
                eprintln!("Import failed: {}", e);
                std::process::exit(1);
            }
            println!("Import completed successfully");
            return;
        }
        Commands::Auth(auth_args) => {
            run_auth(auth_args, config_path);
        }
    }
}

fn run_auth(args: AuthArgs, config_path: PathBuf) {
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
    let force_reauth = args.force_reauth || settings.force_reauth.unwrap_or_default();
    let check_session = settings.check_session.unwrap_or(true);

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
        if let Some(choice) = skim_pick("Select Profile", rows) {
            let parts: Vec<&str> = choice.split('|').map(|s| s.trim()).collect();
            chosen_client = Some(parts[0].to_string());
            chosen_account = Some(parts[1].to_string());
            chosen_role = Some(parts[2].to_string());
        }
    } else {
        if chosen_client.is_none() {
            chosen_client = select_unique_values(&profiles, |p| p.client.clone(), "Select Client");
        }
        if let (Some(client), None) = (&chosen_client, &chosen_account) {
            chosen_account = select_filtered_values(
                &profiles,
                |p| &p.client == client,
                |p| p.account.clone(),
                "Select Account",
            );
        }
        if let (Some(client), Some(account), None) = (&chosen_client, &chosen_account, &chosen_role)
        {
            chosen_role = select_filtered_values(
                &profiles,
                |p| &p.client == client && &p.account == account,
                |p| p.role.clone(),
                "Select Role",
            );
        }
    }

    let (Some(client), Some(account), Some(role)) = (chosen_client, chosen_account, chosen_role)
    else {
        eprintln!("Selection incomplete");
        std::process::exit(1);
    };

    let Some(profile) = profiles
        .iter()
        .find(|p| p.client == client && p.account == account && p.role == role)
    else {
        eprintln!("No matching profile found");
        std::process::exit(1);
    };

    if args.console {
        if let Err(e) = aws::open_console(
            &profile.sso_start_url,
            &profile.sso_account_id,
            &profile.sso_role_name,
            settings.browser.as_deref(),
        ) {
            eprintln!("{}", e);
            std::process::exit(1);
        }
    } else {
        if let Err(e) = aws::login_to_profile(
            &profile.name,
            force_reauth,
            check_session,
            settings.browser.as_deref(),
        ) {
            eprintln!("{}", e);
            std::process::exit(1);
        }
    }

    let max_recent = settings.max_recent_profiles.unwrap_or(100);
    save_recent_profile(&profile.name, max_recent);

    if set_default {
        aws::set_default_profile(&profile.name);
    }
}
