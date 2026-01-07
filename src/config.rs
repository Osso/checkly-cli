use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Config {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub account_id: Option<String>,
}

fn config_dir() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("checkly-cli")
}

fn config_path() -> PathBuf {
    config_dir().join("config.json")
}

pub fn load_config() -> Result<Config> {
    let path = config_path();
    if path.exists() {
        let content = fs::read_to_string(&path)
            .with_context(|| format!("Failed to read config from {:?}", path))?;
        return Ok(serde_json::from_str(&content)?);
    }
    Ok(Config::default())
}

pub fn save_config(config: &Config) -> Result<()> {
    let dir = config_dir();
    fs::create_dir_all(&dir)
        .with_context(|| format!("Failed to create config directory {:?}", dir))?;
    let path = config_path();
    fs::write(&path, serde_json::to_string_pretty(config)?)
        .with_context(|| format!("Failed to write config to {:?}", path))?;
    Ok(())
}
