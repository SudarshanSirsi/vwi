use crate::logger;
use crate::models::Config;
use anyhow::{Context, Result};
use std::path::PathBuf;

/// Returns the expected path to the user's config file:
/// %APPDATA%\vwi\config.toml (e.g. C:\Users\<name>\AppData\Roaming\vwi\config.toml)
/// If the platform has no standard config directory, we fall back to the current
/// working directory so the app can still start.
pub fn config_path() -> PathBuf {
    let mut path = dirs::config_dir().unwrap_or_else(|| std::env::current_dir().unwrap());
    path.push("vwi");
    path.push("config.toml");
    path
}

/// Loads and parses the TOML configuration file.
///
/// If the file does not exist we return Config::default() so the app can
/// start with an empty project list.  This lets a new user see the overlay
/// immediately and understand they need to create a config.
///
/// We print the resolved path to the console for troubleshooting; many
/// Windows users are unfamiliar with %APPDATA% paths.
pub fn load_config() -> Result<Config> {
    let path = config_path();
    logger::info(&format!("Looking for config at: {:?}", path));
    if !path.exists() {
        logger::info("Config not found, using defaults (no projects).");
        return Ok(Config::default());
    }
    logger::info("Config found, loading...");
    let content = std::fs::read_to_string(&path)
        .with_context(|| format!("Failed to read config: {:?}", path))?;
    let config: Config = toml::from_str(&content)
        .with_context(|| format!("Failed to parse config: {:?}", path))?;
    Ok(config)
}
