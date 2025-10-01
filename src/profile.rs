use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct Profile {
    pub name: String,
    pub client: String,
    pub account: String,
    pub role: String,
}

pub fn load_profiles(config_path: &PathBuf) -> Vec<Profile> {
    let contents = fs::read_to_string(config_path).unwrap_or_default();
    let mut profiles = Vec::new();

    for line in contents.lines() {
        if let Some(profile_name) = line.strip_prefix("[profile ").and_then(|s| s.strip_suffix(']')) {
            if let Some(profile) = parse_profile_name(profile_name) {
                profiles.push(profile);
            }
        }
    }

    profiles
}

fn parse_profile_name(name: &str) -> Option<Profile> {
    let parts: Vec<&str> = name.split('-').collect();
    if parts.len() < 3 {
        return None;
    }

    let client = parts[0].to_string();
    let account = parts[1].to_string();
    let role = parts[2..].join("-");

    Some(Profile {
        name: name.to_string(),
        client,
        account,
        role,
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