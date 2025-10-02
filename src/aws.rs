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

pub fn login_to_profile(profile_name: &str, force_reauth: bool, check_session: bool) -> Result<(), String> {
    if check_session && !force_reauth && check_sso_session(profile_name) {
        println!("Profile {} already has a valid session", profile_name);
        return Ok(());
    }
    
    println!("Logging into AWS profile: {}", profile_name);
    let status = Command::new("aws")
        .arg("sso")
        .arg("login")
        .arg("--profile")
        .arg(profile_name)
        .status()
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
