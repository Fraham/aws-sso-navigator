use ini::Ini;
use std::collections::BTreeSet;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct Profile {
    pub name: String,
    pub client: String,
    pub account: String,
    pub role: String,
    //pub sso_session: String,
    pub sso_account_id: String,
    pub sso_role_name: String,
    pub sso_start_url: String,
}

pub fn load_profiles(config_path: &PathBuf) -> Vec<Profile> {
    let Ok(ini) = Ini::load_from_file(config_path) else {
        return Vec::new();
    };

    let mut profiles = Vec::new();

    for (section_name, properties) in ini.iter() {
        if let Some(section_name) = section_name {
            if let Some(profile_name) = section_name.strip_prefix("profile ") {
                if let Some(profile) = parse_profile(profile_name, properties, &ini) {
                    profiles.push(profile);
                }
            }
        }
    }

    profiles
}

fn parse_profile(name: &str, properties: &ini::Properties, ini: &Ini) -> Option<Profile> {
    let parts: Vec<&str> = name.split('-').collect();
    if parts.len() < 3
        || !properties.contains_key("sso_session")
        || !properties.contains_key("sso_account_id")
        || !properties.contains_key("sso_role_name")
    {
        return None;
    }

    let sso_session_name = &properties["sso_session"];
    let sso_session_section = ini.section(Some(&format!("sso-session {}", sso_session_name)))?;
    let sso_start_url = sso_session_section.get("sso_start_url")?;

    let client = parts[0].to_string();
    let account = parts[1].to_string();
    let role = parts[2..].join("-");

    Some(Profile {
        name: name.to_string(),
        client,
        account,
        role,
        //sso_session: properties["sso_session"].to_string(),
        sso_account_id: properties["sso_account_id"].to_string(),
        sso_role_name: properties["sso_role_name"].to_string(),
        sso_start_url: sso_start_url.to_string(),
    })
}

pub fn select_unique_values<F>(profiles: &[Profile], extractor: F, prompt: &str) -> Option<String>
where
    F: Fn(&Profile) -> String,
{
    let options: Vec<String> = profiles
        .iter()
        .map(extractor)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    crate::ui::skim_pick(prompt, options)
}

pub fn select_filtered_values<F, P>(
    profiles: &[Profile],
    filter: P,
    extractor: F,
    prompt: &str,
) -> Option<String>
where
    F: Fn(&Profile) -> String,
    P: Fn(&Profile) -> bool,
{
    let filtered: Vec<_> = profiles.iter().filter(|p| filter(p)).cloned().collect();
    select_unique_values(&filtered, extractor, prompt)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[test]
    fn test_load_profiles_empty_file() {
        let temp_file = NamedTempFile::new().unwrap();
        let profiles = load_profiles(&temp_file.path().to_path_buf());
        assert!(profiles.is_empty());
    }

    #[test]
    fn test_load_profiles_valid_format() {
        let temp_file = NamedTempFile::new().unwrap();
        let mut ini = Ini::new();
        
        ini.with_section(Some("sso-session example"))
            .set("sso_start_url", "https://example.com");
        
        ini.with_section(Some("profile client1-dev-admin"))
            .set("sso_session", "example")
            .set("sso_account_id", "123456789012")
            .set("sso_role_name", "AdministratorAccess");
        
        ini.with_section(Some("profile client2-prod-readonly"))
            .set("sso_session", "example")
            .set("sso_account_id", "987654321098")
            .set("sso_role_name", "ReadOnlyAccess");
        
        ini.write_to_file(temp_file.path()).unwrap();

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
        let temp_file = NamedTempFile::new().unwrap();
        let mut ini = Ini::new();
        
        ini.with_section(Some("sso-session example"))
            .set("sso_start_url", "https://example.com");
        
        ini.with_section(Some("profile invalid"))
            .set("sso_session", "example");
        
        ini.with_section(Some("profile client-dev"))
            .set("sso_session", "example");
        
        ini.with_section(Some("profile valid-dev-admin"))
            .set("sso_session", "example")
            .set("sso_account_id", "123456789012")
            .set("sso_role_name", "AdministratorAccess");
        
        ini.write_to_file(temp_file.path()).unwrap();

        let profiles = load_profiles(&temp_file.path().to_path_buf());
        assert_eq!(profiles.len(), 1);
        assert_eq!(profiles[0].name, "valid-dev-admin");
    }

    #[test]
    fn test_load_profiles_multi_part_role() {
        let temp_file = NamedTempFile::new().unwrap();
        let mut ini = Ini::new();
        
        ini.with_section(Some("sso-session example"))
            .set("sso_start_url", "https://example.com");
        
        ini.with_section(Some("profile client-dev-power-user-access"))
            .set("sso_session", "example")
            .set("sso_account_id", "123456789012")
            .set("sso_role_name", "PowerUserAccess");
        
        ini.write_to_file(temp_file.path()).unwrap();

        let profiles = load_profiles(&temp_file.path().to_path_buf());
        assert_eq!(profiles.len(), 1);
        assert_eq!(profiles[0].role, "power-user-access");
    }
}
