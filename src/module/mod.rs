use std::fs;
use std::fs::File;
use std::path::PathBuf;
use anyhow::Error;
use chksum::Chksum;
use chksum::prelude::HashAlgorithm;
use log::{error, info, warn};
use serde::{Deserialize, Serialize};
use crate::module::install::{InstallAction, InstalledAction};
use crate::module::install::shell::Shell;
use crate::module::reader::ModuleConfig;
use crate::output;

mod reader;
pub mod repository;
pub mod install;

/// File declaring the module config
const MODULE_CONFIG: &str = "module.yml";

#[derive(Serialize, Deserialize)]
pub struct Module {

    pub path: PathBuf,
    repository: String,
    pub qualifier: ModuleQualifier,

    name: String,
    description: String,
    version: String,

    install: Vec<InstallAction>

}

impl Module {

    /// Creates a module instance based on a path
    pub fn create(repo: &String, dir: &PathBuf) -> anyhow::Result<Option<Self>> {
        // Module has to be directory
        if !dir.is_dir() { return Ok(None) }

        // Module has to contain a module.yml file
        let mut config = dir.clone();
        config.push(MODULE_CONFIG);
        if !config.exists() || !config.is_file() { return Ok(None) }

        // Read config file
        let config = ModuleConfig::load(&config)?;

        Ok(Some(Self {
            repository: repo.clone(),
            path: dir.clone(),

            qualifier: ModuleQualifier {
                provide: config.provides,
                alias: config.alias,
                dir: dir.file_name().ok_or_else(|| Error::msg("Failed to get module directory name!"))?.to_string_lossy().to_string()
            },

            name: config.name,
            description: config.description,
            version: config.version,

            install: config.install
        }))
    }

    pub fn unique_qualifier(&self) -> String {
        format!("{}/{}", &self.repository, &self.qualifier.name())
    }

    pub fn install(&self, shell: &Shell) -> anyhow::Result<Vec<InstalledAction>>{
        let mut installed = vec![];

        let mut failure = false;
        'install_loop: for (i, action) in self.install.iter().enumerate() {

            info!("");
            output::start_section(&format!("Starting {}action{} ({}/{})",
                                           if action.is_optional() { "optional " } else { "" },
                                           action.get_title().clone().map(|title| format!(" '{}'", &title)).unwrap_or_else(|| "".to_string()),
                                           i + 1. self.install.len()));

            match action.install(&shell, &self.path) {
                Ok(install) => { installed.push(install) }
                Err(e) => {

                    if action.is_optional() {
                        warn!("Failed to execute optional action: {}", e);
                    } else {
                        error!("Failed to execute mandatory action: {}", e);
                        failure = true;
                        break 'install_loop;
                    }
                }
            }
        }

        if failure {
            info!("");
            error!("Failed to perform all mandatory actions, uninstalling completed ones...");

            for action in installed {
                if let Err(e) = action.uninstall(&shell, &self.path) {
                    warn!("Failed to execute uninstall action: {}", e);
                }
            }

            Err(Error::msg("at least one install action failed to execute"))
        } else {
            Ok(installed)
        }
    }

    pub fn current_checksum(&self) -> String {
        fs::read_dir(&self.path).map(|mut f| {
            f.chksum(HashAlgorithm::SHA1).map(|digest| format!("{:x}", digest)).unwrap_or_else(|_| "checksum-making-failed".to_string())
        }).unwrap_or_else(|_| "checksum-file-reading-failed".to_string())
    }
}

#[derive(Serialize, Deserialize)]
pub struct ModuleQualifier {
    /// Name of the directory
    dir: String,
    /// Alias defined in the config
    alias: Option<String>,
    /// Provides defined in the config
    provide: Option<String>
}

impl ModuleQualifier {

    /// Returns whether the module provides the named qualifier
    pub fn does_provide(&self, qualifier: &str) -> bool {

        // Provides module
        if let Some(provide) = &self.provide {
            if provide == qualifier { return true }
        }

        // Is the module
        self.name() == qualifier
    }

    /// Returns whether the module is it
    pub fn is(&self, qualifier: &str) -> bool {
        self.name() == qualifier
    }

    /// Returns qualifying name for module
    pub fn name(&self) -> &String {
        if let Some(alias) = &self.alias {
            alias
        } else {
            &self.dir
        }
    }

    /// Returns alternative providing name
    pub fn provide(&self) -> &Option<String> {
        &self.provide
    }
}

