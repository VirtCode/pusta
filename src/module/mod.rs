use std::fmt::format;
use std::fs;
use std::fs::File;
use std::path::{Path, PathBuf};
use anyhow::{anyhow, Context, Error};
use chksum::Chksum;
use chksum::prelude::HashAlgorithm;
use colored::Colorize;
use log::{error, info, warn};
use serde::{Deserialize, Serialize};
use crate::jobs::Job;
use crate::module::qualifier::ModuleQualifier;
use crate::module::repository::Repository;
use crate::output;
use crate::output::end_section;
use crate::registry::index::Indexable;

pub mod repository;
pub mod install;
pub mod qualifier;

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

#[derive(Serialize, Deserialize, Clone)]
pub struct Module {
    pub path: PathBuf,
    pub qualifier: ModuleQualifier,
    pub dependencies: Vec<String>,
    checksum: String,

    pub name: String,
    pub description: String,
    pub author: Option<String>,
    pub version: String,

    jobs: Vec<Job>

}

impl Module {

    // Loads a module from a directory
    pub fn load(location: &Path, parent: &Repository) -> anyhow::Result<Self> {

        // Load config file
        let mut config = location.to_owned();
        config.push(MODULE_CONFIG);
        let config: ModuleConfig = serde_yaml::from_reader(File::open(&config).context("Failed to open config file, does it exist?")?)
            .map_err(|f| anyhow!("Failed to read config file ({})", f.to_string()))?;

        // Calculate current checksum
        let checksum = fs::read_dir(location).context("Failed to read dir for checksum").and_then(|mut f| {
            f.chksum(HashAlgorithm::SHA1).map(|digest| format!("{:x}", digest)).context("Failed to calculate checksum")
        })?;

        // Read dependencies
        let dependencies: Vec<String> = config.depends.map(|s| s.split(' ').map(str::to_owned).collect()).unwrap_or_default();


        Ok(Self {
            path: location.to_owned(),
            qualifier: ModuleQualifier::new(parent.name.clone(), location, config.alias, config.provides),
            checksum,
            dependencies,
            name: config.name,
            description: config.description,
            author: config.author,
            version: config.version,

            jobs: config.jobs
        })
    }

    pub fn equals_jobs(&self, other: &Self) -> bool {
        self.jobs == other.jobs
    }

    pub fn up_to_date(&self, new: &Self) -> bool {
        self.checksum == new.checksum
    }
}

impl Indexable for Module {
    fn dependencies(&self) -> &Vec<String> {
        &self.dependencies
    }

    fn qualifier(&self) -> &ModuleQualifier {
        &self.qualifier
    }
}