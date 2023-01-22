use std::fs;
use std::fs::File;
use std::ops::Not;
use std::os::unix::fs::chroot;
use std::path::PathBuf;
use anyhow::Context;
use log::{debug, error, info, warn};
use serde::{Deserialize, Serialize};

pub const CONFIG_FILE: &str = "~/.config/pusta/config.yml";

#[derive(Deserialize, Serialize, Clone)]
pub struct Config {

    pub repositories: ConfigRepository,
    pub security: ConfigSecurity,
    pub log: ConfigLog,
    pub system: ConfigShell

}

#[derive(Deserialize, Serialize, Clone)]
pub struct ConfigRepository {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub main: Option<String>,
    pub strict_qualifying: bool
}

#[derive(Deserialize, Serialize, Clone)]
pub struct ConfigShell {
    pub root_elevator: String,
    pub file_previewer: String,
    pub package_manager: ConfigPackage
}

#[derive(Deserialize, Serialize, Clone)]
pub struct ConfigPackage {
    pub root: bool,
    pub install: String,
    pub remove: String
}

#[derive(Deserialize, Serialize, Clone)]
pub struct ConfigLog {
    pub log_files: bool,
    pub verbose: bool
}

#[derive(Deserialize, Serialize, Clone)]
pub enum ConfirmStrategy {
    #[serde(rename="false", alias="false", alias="no", alias="No", alias="False")]
    No,
    #[serde(rename="true", alias="true", alias="yes", alias="Yes", alias="True")]
    Yes,
    #[serde(rename="root", alias="root", alias="Root")]
    Root
}

#[derive(Deserialize, Serialize, Clone)]
pub enum PreviewStrategy {
    #[serde(rename="always", alias="always", alias="Always")]
    Always,
    #[serde(rename="root", alias="root", alias="Root")]
    Root,
    #[serde(rename="never", alias="never", alias="Never")]
    Never,
    #[serde(rename="ask", alias="ask", alias="Ask")]
    Ask,
    #[serde(rename="ask-root", alias="ask-root", alias="Ask-Root")]
    AskRoot
}

#[derive(Deserialize, Serialize, Clone)]
pub struct ConfigSecurity {
    #[serde(skip_serializing_if = "is_false", default)]
    pub extra_confirm_everything: bool,

    pub confirm_packages: bool,
    pub preview_scripts: PreviewStrategy,
    pub confirm_scripts: ConfirmStrategy,
    pub confirm_files: ConfirmStrategy
}
fn is_false(value: &bool) -> bool { !*value }


impl Default for Config {
    fn default() -> Self {
        Config {
            repositories: ConfigRepository {
                main: None,
                strict_qualifying: false
            },
            log: ConfigLog {
                log_files: true,
                verbose: false
            },
            system: ConfigShell {
                root_elevator: "sudo $COMMAND$".to_string(),
                file_previewer: "less $FILE$".to_string(),
                package_manager: ConfigPackage {
                    root: false,
                    install: "echo 'Installing package $PACKAGE$'".to_string(),
                    remove: "echo 'Removing package $PACKAGE$'".to_string()
                }
            },
            security: ConfigSecurity {
                extra_confirm_everything: false,
                confirm_packages: true,
                preview_scripts: PreviewStrategy::Ask,
                confirm_scripts: ConfirmStrategy::Yes,
                confirm_files: ConfirmStrategy::Root
            }
        }
    }
}

impl Config {
    pub fn read() -> Self {
        debug!("Reading config file");

        if let Ok(c) = {
            let path = PathBuf::from(shellexpand::tilde(CONFIG_FILE).to_string());

            File::open(&path).map_err(anyhow::Error::new).and_then(|f| serde_yaml::from_reader(f).map_err(anyhow::Error::new)).map_err(|e| {
                eprintln!("Error occurred: {}", e)
            })
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
