use std::collections::HashMap;
use std::path::PathBuf;

/// Load station aliases from ~/.config/sncf/aliases.toml
/// Returns empty map if file doesn't exist or can't be parsed.
pub fn load_aliases() -> HashMap<String, String> {
    let path = aliases_path();
    let content = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(_) => return HashMap::new(),
    };
    toml::from_str::<HashMap<String, String>>(&content).unwrap_or_default()
}

fn aliases_path() -> PathBuf {
    dirs_fallback()
        .join(".config")
        .join("sncf")
        .join("aliases.toml")
}

fn dirs_fallback() -> PathBuf {
    std::env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."))
}
