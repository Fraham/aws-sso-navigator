use std::process::Command;
use std::fs;
use std::path::PathBuf;
use dirs::home_dir;
use serde::Deserialize;
use ini::Ini;

#[derive(Deserialize)]
struct AccountList {
    #[serde(rename = "accountList")]
    account_list: Vec<Account>,
}

#[derive(Deserialize)]
struct Account {
    #[serde(rename = "accountId")]
    account_id: String,
    #[serde(rename = "accountName")]
    account_name: String,
}

#[derive(Deserialize)]
struct RoleList {
    #[serde(rename = "roleList")]
    role_list: Vec<Role>,
}

#[derive(Deserialize)]
struct Role {
    #[serde(rename = "roleName")]
    role_name: String,
    #[serde(rename = "accountId")]
    account_id: String,
}

#[derive(Deserialize)]
struct SsoToken {
    #[serde(rename = "accessToken")]
    access_token: String,
}

pub fn import_profiles(sso_session: &str, config_path: &PathBuf) -> Result<(), String> {
    // Login to SSO session
    let status = Command::new("aws")
        .args(["sso", "login", "--sso-session", sso_session])
        .status()
        .map_err(|e| format!("Failed to execute aws: {}", e))?;

    if !status.success() {
        return Err("AWS SSO login failed".to_string());
    }

    // Get access token from cache
    let cache_dir = home_dir().unwrap().join(".aws/sso/cache");
    let mut cache_files: Vec<_> = fs::read_dir(&cache_dir)
        .map_err(|e| format!("Failed to read cache directory: {}", e))?
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.path().extension().map_or(false, |ext| ext == "json"))
        .collect();

    cache_files.sort_by_key(|entry| entry.metadata().unwrap().modified().unwrap());
    
    let token_file = cache_files.last()
        .ok_or("No token file found")?;

    let token_content = fs::read_to_string(token_file.path())
        .map_err(|e| format!("Failed to read token file: {}", e))?;
    
    let token: SsoToken = serde_json::from_str(&token_content)
        .map_err(|e| format!("Failed to parse token: {}", e))?;

    // Get region from sso-session
    let ini = Ini::load_from_file(config_path)
        .map_err(|e| format!("Failed to load config: {}", e))?;
    
    let sso_region = ini.section(Some(&format!("sso-session {}", sso_session)))
        .and_then(|section| section.get("sso_region"))
        .ok_or("SSO region not found in config")?;

    // List accounts
    let accounts_output = Command::new("aws")
        .args(["sso", "list-accounts", "--region", sso_region, "--access-token", &token.access_token])
        .output()
        .map_err(|e| format!("Failed to list accounts: {}", e))?;

    if !accounts_output.status.success() {
        return Err("Failed to list accounts".to_string());
    }

    let accounts: AccountList = serde_json::from_slice(&accounts_output.stdout)
        .map_err(|e| format!("Failed to parse accounts: {}", e))?;

    println!("Found {} accounts", accounts.account_list.len());

    let mut config_content = String::new();

    for (i, account) in accounts.account_list.iter().enumerate() {
        println!("[{}/{}] Processing account: {}", i + 1, accounts.account_list.len(), account.account_name);

        let roles_output = Command::new("aws")
            .args(["sso", "list-account-roles", "--region", sso_region, "--access-token", &token.access_token, "--account-id", &account.account_id])
            .output()
            .map_err(|e| format!("Failed to list roles: {}", e))?;

        if !roles_output.status.success() {
            continue;
        }

        let roles: RoleList = serde_json::from_slice(&roles_output.stdout)
            .map_err(|e| format!("Failed to parse roles: {}", e))?;

        for role in &roles.role_list {
            let profile_name = format!("{}-{}-{}", sso_session, account.account_name.replace(' ', "").replace('-', "_"), role.role_name.replace('-', "_"));
            
            if ini.section(Some(&format!("profile {}", profile_name))).is_some() {
                println!("Profile {} already exists, skipping", profile_name);
                continue;
            }
            
            config_content.push_str(&format!(
                "\n[profile {}]\nsso_session = {}\nsso_account_id = {}\nsso_role_name = {}\nregion = {}\noutput = json\n",
                profile_name, sso_session, role.account_id, role.role_name, sso_region
            ));

            println!("Added profile: {}", profile_name);
        }
    }

    fs::write(config_path, fs::read_to_string(config_path).unwrap_or_default() + &config_content)
        .map_err(|e| format!("Failed to write config: {}", e))?;

    Ok(())
}