use std::fmt::format;
use std::fs;
use std::fs::File;
use std::path::{Path, PathBuf};
use anyhow::{anyhow, Context, Error};
use chksum::chksum;
use chksum::hash::SHA1;
use colored::Colorize;
use log::{debug, error, info, warn};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use crate::jobs::Job;
use crate::module::qualifier::ModuleQualifier;
use crate::module::repository::Repository;
use crate::output;
use crate::output::end_section;
use crate::variables::Variable;
use crate::registry::index::Indexable;

pub mod repository;
pub mod qualifier;
pub mod change;
pub mod install;

/// File declaring the module config
const MODULE_CONFIG: &str = "module.yml";

#[derive(Deserialize, JsonSchema)]
#[schemars(title = "Module", deny_unknown_fields)]
pub struct ModuleConfig {
    /// display name
    name: String,
    /// describes the purpose of the module when it is installed
    description: String,
    /// primary author
    author: Option<String>,
    // support numeric types as a version is very likely to be recognized as number by the lsp
    /// current version of the module
    #[schemars(extend("type" = [ "string", "number" ]))]
    version: String,

    /// overrides the module name, by default is directory name
    alias: Option<String>,
    /// this module provides this module too when installed
    provides: Option<String>,
    /// list of other modules this depends on
    depends: Option<String>,
    /// precedence this module has when compared to other modules, mainly used for injections
    precedence: Option<u32>,

    /// list of jobs to install
    jobs: Vec<Job>,

    /// variables used only for this module
    variables: Option<Variable>,

    /// variables injected into the global variable tree when this module is installed
    injections: Option<Variable>
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

    jobs: Vec<Job>,

    variables: Option<Variable>,

    pub injections: Option<Variable>,
    pub precedence: Option<u32>
}

impl Module {

    pub fn try_load(directory: &Path, parent: &Repository) -> anyhow::Result<Option<Self>> {
        // Validating directory
        if !directory.is_dir() { return Ok(None); }

        let mut config = directory.to_owned();
        config.push(MODULE_CONFIG);

        if !config.exists() || !config.is_file() { return Ok(None); }

        debug!("Loading module from '{}'", config.to_string_lossy());

        // Reading config
        let config: ModuleConfig = serde_yaml::from_reader(File::open(&config).context("Failed to open config file, does it exist?")?)
            .map_err(|f| anyhow!("Failed to read config file ({})", f.to_string()))?;

        let dependencies: Vec<String> = config.depends.map(|s| s.split(' ').map(str::to_owned).collect()).unwrap_or_default();

        // Calculate current checksum
        let dir = fs::read_dir(directory).context("Failed to read dir for checksum")?;
        let checksum = chksum::<SHA1, _>(dir).context("Failed to calculate checksum")?.to_hex_lowercase();

        let qualifier = ModuleQualifier::new(parent.name.clone(), directory, config.alias, config.provides);
        if !qualifier.legal() {
            return Err(anyhow!("Module qualifier contains illegal characters"));
        }

        Ok(Some(Self {
            path: directory.to_owned(),
            qualifier,
            checksum,
            dependencies,
            name: config.name,
            description: config.description,
            author: config.author,
            version: config.version,
            precedence: config.precedence,

            jobs: config.jobs,
            variables: config.variables,
            injections: config.injections
        }))
    }

    pub fn equals_jobs(&self, other: &Self) -> bool {
        self.jobs == other.jobs
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
