use std::fmt::format;
use std::fs;
use std::fs::File;
use std::path::{Path, PathBuf};
use anyhow::{Context, Error};
use chksum::Chksum;
use chksum::prelude::HashAlgorithm;
use colored::Colorize;
use log::{error, info, warn};
use serde::{Deserialize, Serialize};
use crate::jobs::Job;
use crate::module::install::{InstallAction, InstalledAction};
use crate::module::install::shell::Shell;
use crate::module::qualifier::ModuleQualifier;
use crate::module::reader::ModuleConfig as OtherModuleConfig;
use crate::module::repository::Repository;
use crate::output;
use crate::output::end_section;

mod reader;
pub mod repository;
pub mod install;
mod qualifier;

/// File declaring the module config
const MODULE_CONFIG: &str = "module.yml";

#[derive(Deserialize)]
pub struct ModuleConfig {
    name: String,
    description: String,
    author: Option<String>,
    version: String,

    alias: Option<String>,
    provides: Option<String>,
    depends: Option<String>,

    jobs: Vec<Job>

    // actions, variables, lists
}

#[derive(Serialize, Deserialize)]
pub struct Module {
    pub path: PathBuf,
    pub qualifier: ModuleQualifier,
    checksum: String,

    name: String,
    description: String,
    author: Option<String>,
    version: String,

    jobs: Vec<Job>

}

impl Module {

    // Loads a module from a directory
    pub fn load(location: &Path, parent: &Repository) -> anyhow::Result<Self> {

        // Load config file
        let mut config = location.to_owned();
        config.push(MODULE_CONFIG);
        let config: ModuleConfig = serde_yaml::from_reader(File::open(&config).context("Failed to open config file, does it exist?")?).context("Failed to read config file, make sure every mandatory attribute is provided")?;

        // Calculate current checksum
        let checksum = fs::read_dir(location).context("Failed to read dir for checksum").and_then(|mut f| {
            f.chksum(HashAlgorithm::SHA1).map(|digest| format!("{:x}", digest)).context("Failed to calculate checksum")
        })?;


        Ok(Self {
            path: location.to_owned(),
            qualifier: ModuleQualifier::new(parent.name.clone(), location, config.alias, config.provides),
            checksum,

            name: config.name,
            description: config.description,
            author: config.author,
            version: config.version,

            jobs: config.jobs
        })
    }

    pub fn install(&self, shell: &Shell) -> anyhow::Result<Vec<InstalledAction>>{
        let mut installed = vec![];

        let mut failure = false;
        'install_loop: for (i, action) in self.install.iter().enumerate() {

            info!("");
            output::start_section(&format!("{current}/{total} - Starting {title}{optional}",
                current = i + 1,
                total = self.install.len(),
                title = action.get_title().clone().map(|title| format!("to {}", &title.italic())).unwrap_or_else(|| "next action".to_string()),
                optional = if action.is_optional() { " (optional)".dimmed().to_string() } else { "".to_string() },
            ));

            match action.install(shell, &self.path) {
                Ok(install) => {
                    installed.push(install);

                    end_section(true, &format!("Successfully completed action {current}/{total}", current = i + 1, total = self.install.len()));
                }
                Err(e) => {

                    if action.is_optional() {
                        warn!("Failed to execute optional action: {}", e);
                    } else {
                        error!("Failed to execute mandatory action: {}", e);
                    }

                    end_section(false, &format!("Failed to complete action {current}/{total}", current = i + 1, total = self.install.len()));

                    if !action.is_optional() {
                        failure = true;
                        break 'install_loop;
                    }
                }
            }
        }

        // Reverse array to undo the other way round
        installed.reverse();

        if failure {
            info!("\n");
            error!("Failed to complete all mandatory actions, installation failed");
            info!("Undoing already completed actions now...");

            if uninstall(installed, &self.path, shell).is_err() {
                info!("");
                error!("Failed to undo all completed actions, some things may still be left on your system")
            }

            Err(Error::msg("At least one mandatory install action failed to execute"))
        } else {
            Ok(installed)
        }
    }
}

pub fn uninstall(actions: Vec<InstalledAction>, cache: &Path, shell: &Shell) -> anyhow::Result<()>{

    let mut success = true;

    for (i, action) in actions.iter().enumerate() {

        info!("");
        output::start_section(&format!("{current}/{total} - Starting to undo '{title}'",
                                       current = i + 1,
                                       total = actions.len(),
                                       title = action.get_title().clone().unwrap_or_else(|| "an untitled action".to_string()),
        ));

        if let Err(e) = action.uninstall(shell, cache) {
            success = false;
            error!("Failed to undo action: {}", e);
            output::end_section(false, &format!("Undoing of action {}/{} failed", i + 1, actions.len()));
        } else {
            output::end_section(true, &format!("Action {}/{} was undone successfully", i + 1, actions.len()));
        }
    }

    if !success { Err(Error::msg("Not all actions could undo themselves properly")) }
    else { Ok(()) }
}