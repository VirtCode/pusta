use std::{env};
use std::fs::File;
use std::path::PathBuf;
use anyhow::{anyhow, Context};
use log::debug;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use crate::registry::cache;
use crate::variables;

pub const DEFAULT_PARENT: &str = "~/.config";
pub const DEFAULT_FILE: &str = "/config.yml";

/// Finds the current config directory XDG_CONFIG_HOME/pusta
pub fn config_dir() -> String {
    let parent = match env::var("XDG_CONFIG_HOME") {
        Ok(s) => { s }
        Err(_) => { DEFAULT_PARENT.to_owned() }
    };

    parent + "/pusta"
}

/// Finds the current config file ([`config_dir`][`DEFAULT_FILE`])
pub fn config_file() -> String {
    config_dir() + DEFAULT_FILE
}

/// This struct contains the main config with default values
#[derive(Deserialize, Clone, JsonSchema)]
#[schemars(deny_unknown_fields)]
pub struct Config {
    #[serde(default = "cache::default_cache_dir")]
    pub cache_dir: String,

    #[serde(default = "variables::default_system_variables")]
    pub system_variables: String,

    #[serde(default)]
    pub system: ConfigShell,

    #[serde(default)]
    pub security: ConfigSecurity,
}

impl Config {
    pub fn read() -> anyhow::Result<Self> {
        debug!("Reading config file");

        let path = PathBuf::from(shellexpand::tilde(&config_file()).to_string());

        if path.exists() {
            File::open(&path).map_err(|e| anyhow!(e))
                .and_then(|f| serde_yaml::from_reader(f).context("Failed to deserialize config"))
        } else {
            debug!("Config does not exist, using default values");
            Ok(Default::default())
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            cache_dir: cache::default_cache_dir(),
            system_variables: variables::default_system_variables(),
            system: Default::default(),
            security: Default::default()
        }
    }
}


/// This struct contains configuration about the current system and shell
#[derive(Deserialize, Clone, JsonSchema)]
#[schemars(title = "System")]
pub struct ConfigShell {
    #[serde(default="ConfigShell::root_elevator_default")]
    pub root_elevator: String,
    #[serde(default="ConfigShell::file_previewer_default")]
    pub file_previewer: String,
    #[serde(default="ConfigShell::clean_terminal_default")]
    pub clean_terminal: bool,
    #[serde(default)]
    pub package_manager: ConfigPackage,
    pub default_directory: Option<String>
}

impl ConfigShell {
    /// The default value for the root elevator
    pub fn root_elevator_default() -> String {
        "sudo".to_owned()
    }

    /// The default value for the file previewer (less, because it is a gnu coreutil and thus on (almost) every linux distro)
    pub fn file_previewer_default() -> String {
        "less".to_owned()
    }

    /// The default terminal cleaning setting, is true as we use sudo
    pub fn clean_terminal_default() -> bool {
        true
    }
}

impl Default for ConfigShell {
    fn default() -> Self {
        ConfigShell {
            root_elevator: ConfigShell::root_elevator_default(),
            file_previewer: ConfigShell::file_previewer_default(),
            package_manager: Default::default(),
            clean_terminal: ConfigShell::clean_terminal_default(),
            default_directory: None
        }
    }
}

/// This struct contains configuration about the package manager, having dummy defaults
#[derive(Deserialize, Clone, JsonSchema)]
pub struct ConfigPackage {
    pub root: bool,
    pub install: String,
    pub remove: String
}

impl Default for ConfigPackage {
    fn default() -> Self {
        Self {
            install: "echo \"Package manager is not configured yet\"; exit 1".to_owned(),
            remove: "echo \"Package manager is not configured yet\"; exit 1".to_owned(),
            root: false
        }
    }
}

impl ConfigPackage {
    pub fn create_install(&self, packages: &Vec<String>) -> String {
        let packages = packages.join(" ");
        self.install.clone().replace("%PACKAGE%", &packages)
    }

    pub fn create_remove(&self, packages: &Vec<String>) -> String {
        let packages = packages.join(" ");
        self.remove.clone().replace("%PACKAGE%", &packages)
    }
}

/// This enum represents a strategy used to confirm changes to the system
#[derive(Deserialize, Clone, JsonSchema)]
pub enum ConfirmStrategy {
    #[serde(rename="false", alias="false", alias="no", alias="No", alias="False")]
    No,
    #[serde(rename="true", alias="true", alias="yes", alias="Yes", alias="True")]
    Yes,
    #[serde(rename="root", alias="root", alias="Root")]
    Root
}

impl Default for ConfirmStrategy {
    fn default() -> Self {
        ConfirmStrategy::Root
    }
}

/// This enum represents a strategy to determine when to preview a script to execute
#[derive(Deserialize, Serialize, Clone, JsonSchema)]
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

impl Default for PreviewStrategy {
    fn default() -> Self {
        PreviewStrategy::AskRoot
    }
}

/// This struct contains guidelines about which actions on the system should be confirmed
#[derive(Deserialize, Clone, JsonSchema)]
#[schemars(title = "Security")]
pub struct ConfigSecurity {
    #[serde(default)]
    pub extra_confirm_everything: bool,

    #[serde(default)]
    pub preview_scripts: PreviewStrategy,

    #[serde(default="ConfigSecurity::confirm_packages_default")]
    pub confirm_packages: bool,
    #[serde(default)]
    pub confirm_execution: ConfirmStrategy,
    #[serde(default)]
    pub confirm_files: ConfirmStrategy
}

impl ConfigSecurity {
    /// Default value whether to confirm package installs
    pub fn confirm_packages_default() -> bool { true }
}

impl Default for ConfigSecurity {
    fn default() -> Self {
        Self {
            confirm_packages: ConfigSecurity::confirm_packages_default(),
            extra_confirm_everything: false,
            preview_scripts: Default::default(),
            confirm_execution: Default::default(),
            confirm_files: Default::default()
        }
    }
}




