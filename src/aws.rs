use std::process::Command;

pub fn login_to_profile(profile_name: &str) -> Result<(), String> {
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
