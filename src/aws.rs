use std::process::Command;

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

pub fn set_default_profile(profile_name: &str) {
    unsafe { std::env::set_var("AWS_PROFILE", profile_name) };
    println!("Set {} as default AWS profile", profile_name);
}

pub fn open_console(sso_start_url: &str, sso_account_id: &str, sso_role_name: &str, browser: Option<&str>) -> Result<(), String> {
    let url = format!("{}/#/console?account_id={}&role_name={}", sso_start_url, sso_account_id, sso_role_name);
    
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
