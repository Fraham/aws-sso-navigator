use std::process::Command;
use std::path::PathBuf;
use ini::Ini;
use crate::profile::load_profiles;

fn check_sso_session(profile_name: &str) -> bool {
    let output = Command::new("aws")
        .args(["sts", "get-caller-identity", "--profile", profile_name])
        .output();
    
    match output {
        Ok(result) => result.status.success(),
        Err(_) => false,
    }
}

pub fn login_to_profile(profile_name: &str, force_reauth: bool, check_session: bool, browser: Option<&str>) -> Result<(), String> {
    if check_session && !force_reauth && check_sso_session(profile_name) {
        println!("Profile {} already has a valid session", profile_name);
        return Ok(());
    }
    
    println!("Logging into AWS profile: {}", profile_name);
    let mut cmd = Command::new("aws");
    cmd.arg("sso")
        .arg("login")
        .arg("--profile")
        .arg(profile_name);
    
    if let Some(browser_path) = browser {
        cmd.env("BROWSER", browser_path);
    }
    
    let status = cmd.status()
        .map_err(|e| format!("Failed to execute aws: {}", e))?;

    if !status.success() {
        return Err("AWS SSO login failed".to_string());
    }

    Ok(())
}

pub fn set_default_profile(profile_name: &str, config_path: &PathBuf) -> Result<(), String> {
    let profiles = load_profiles(config_path);
    profiles.iter()
        .find(|p| p.name == profile_name)
        .ok_or_else(|| format!("Profile {} not found", profile_name))?;
    
    let mut ini = Ini::load_from_file(config_path)
        .map_err(|e| format!("Failed to load AWS config: {}", e))?;
    
    let source_section_name = format!("profile {}", profile_name);
    let source_data: Vec<(String, String)> = ini.section(Some(&source_section_name))
        .ok_or_else(|| format!("Profile {} not found in config", profile_name))?
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();
    
    let mut default_section = ini.with_section(Some("default"));
    for (key, value) in source_data {
        default_section.set(&key, &value);
    }
    
    ini.write_to_file(config_path)
        .map_err(|e| format!("Failed to write AWS config: {}", e))?;
    
    println!("Set {} as default AWS profile", profile_name);
    Ok(())
}

fn normalize_sso_start_url(url: &str) -> &str {
    url.trim_end_matches('/').trim_end_matches('#').trim_end_matches('/')
}

pub fn open_console(sso_start_url: &str, sso_account_id: &str, sso_role_name: &str, browser: Option<&str>) -> Result<(), String> {
    let base_url = normalize_sso_start_url(sso_start_url);
    let url = format!("{}/#/console?account_id={}&role_name={}", base_url, sso_account_id, sso_role_name);
    
    let mut cmd = if let Some(browser_path) = browser {
        Command::new(browser_path)
    } else {
        Command::new("open")
    };
    
    cmd.arg(&url);
    
    let status = cmd.status()
        .map_err(|e| format!("Failed to open browser: {}", e))?;

    if !status.success() {
        return Err("Failed to open console".to_string());
    }

    println!("Opening AWS console: {}", url);
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_sso_start_url() {
        assert_eq!(normalize_sso_start_url("https://account-1.awsapps.com/start"), "https://account-1.awsapps.com/start");
        assert_eq!(normalize_sso_start_url("https://account-2.awsapps.com/start/"), "https://account-2.awsapps.com/start");
        assert_eq!(normalize_sso_start_url("https://account-3.awsapps.com/start/#"), "https://account-3.awsapps.com/start");
        assert_eq!(normalize_sso_start_url("https://account-4.awsapps.com/start/#/"), "https://account-4.awsapps.com/start");
    }
}