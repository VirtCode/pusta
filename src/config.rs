use std::fs;
use std::fs::File;
use std::path::PathBuf;
use anyhow::Context;
use log::{debug, error, info, warn};
use serde::{Deserialize, Serialize};

pub const CONFIG_FILE: &str = "~/.config/pusta/config.yml";

#[derive(Deserialize, Serialize)]
pub struct Config {

    pub repositories: ConfigRepository

}

#[derive(Deserialize, Serialize)]
pub struct ConfigRepository {
    pub main: Option<String>,
    pub strict_qualifying: bool
}

impl Default for Config {
    fn default() -> Self {
        Config {
            repositories: ConfigRepository {
                main: None,
                strict_qualifying: false
            }
        }
    }
}

impl Config {
    pub fn read() -> Self {
        debug!("Reading config file");

        if let Ok(c) = {
            let path = PathBuf::from(shellexpand::tilde(CONFIG_FILE).to_string());

            File::open(&path).map_err(anyhow::Error::new).and_then(|f| serde_yaml::from_reader(f).map_err(anyhow::Error::new))
        } {
            c
        } else {
            info!("Failed to read config file, creating a new one");

            let c = Self::default();
            c.write();

            c
        }
    }

    pub fn write(&self) {
        debug!("Writing config file");

        let path = PathBuf::from(shellexpand::tilde(CONFIG_FILE).to_string());

        if let Some(path) = path.parent() { fs::create_dir_all(path).unwrap_or_else(|e| warn!("Failed to create parent directory for config file: {}", e)); }

        File::create(&path).map_err(anyhow::Error::new).and_then(|f| serde_yaml::to_writer(f, self).map_err(anyhow::Error::new)).unwrap_or_else(|e| warn!("Failed to write config to disk: {}", e));
    }

}
