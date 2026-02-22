use serde::Deserialize;
use std::path::{Path, PathBuf};

const DEFAULT_PORT: u16 = 8200;

fn default_name() -> String {
    let host = hostname::get()
        .ok()
        .and_then(|os| os.into_string().ok())
        .filter(|s| !s.is_empty())
        .unwrap_or_default();
    if host.is_empty() {
        "udlna".to_string()
    } else {
        format!("udlna@{}", host)
    }
}

#[derive(Deserialize, Default, Debug)]
pub struct FileConfig {
    pub port: Option<u16>,
    pub name: Option<String>,
    pub localhost: Option<bool>,
}

#[derive(Debug)]
pub struct Config {
    pub port: u16,
    pub name: String,
    pub paths: Vec<PathBuf>,
    pub localhost: bool,
}

impl Config {
    pub fn resolve(file: Option<FileConfig>, args: &crate::cli::Args) -> Self {
        let file = file.unwrap_or_default();
        Config {
            port: args.port.or(file.port).unwrap_or(DEFAULT_PORT),
            name: args.name.clone().or(file.name).unwrap_or_else(default_name),
            paths: args.paths.clone(),
            localhost: args.localhost || file.localhost.unwrap_or(false),
        }
    }
}

pub fn find_config_file(explicit: Option<&Path>) -> Option<PathBuf> {
    if let Some(path) = explicit {
        return Some(path.to_owned());
    }
    let cwd_config = PathBuf::from("udlna.toml");
    if cwd_config.exists() {
        return Some(cwd_config);
    }
    if let Some(config_dir) = dirs::config_dir() {
        let xdg_config = config_dir.join("udlna").join("config.toml");
        if xdg_config.exists() {
            return Some(xdg_config);
        }
    }
    None
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("failed to read config file: {0}")]
    Io(#[from] std::io::Error),
    #[error("failed to parse config file: {0}")]
    Parse(#[from] toml::de::Error),
}

pub fn load_config(path: &Path) -> Result<FileConfig, ConfigError> {
    let content = std::fs::read_to_string(path)?;
    let config: FileConfig = toml::from_str(&content)?;
    Ok(config)
}
