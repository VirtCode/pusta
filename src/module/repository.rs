use std::borrow::Cow;
use std::ffi::OsStr;
use std::fs;
use std::fs::File;
use std::path::{Path, PathBuf};
use anyhow::{anyhow, Context};
use log::warn;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use crate::module::host::{Host, HOST_CONFIG_FILEENDING};
use crate::module::Module;
use crate::variables::Variable;

pub const REPOSITORY_CONFIG: &str = "pusta.yml";

#[derive(Deserialize, JsonSchema)]
#[schemars(title = "Repository", deny_unknown_fields)]
pub struct RepositoryConfig {
    pub alias: Option<String>,

    pub variables: Option<Variable>
}

#[derive(Serialize, Deserialize)]
pub struct Repository {

    pub location: PathBuf,
    pub name: String,
}

impl Repository {
    pub fn load(folder: &Path, alias: Option<&str>) -> anyhow::Result<Self>{

        // Read config
        let path = folder.join(REPOSITORY_CONFIG);
        let config: RepositoryConfig = serde_yaml::from_reader(
            File::open(&path).with_context(|| format!("Failed to open repository config file at {}", path.to_string_lossy()))?
        ).map_err(|e| anyhow!("failed to parse repository config file at {}: {e:#}", path.to_string_lossy()))?;

        let name = alias.map(|s| s.to_owned())
            .or(config.alias)
            .or_else(|| folder.canonicalize().ok()?.file_name().map(|s| s.to_string_lossy().to_string()))
            .context("Could not find name for repository")?;

        if !legal_name(&name) {
            return Err(anyhow!("Repository name contains illegal characters"));
        }

        Ok(Repository {
            location: fs::canonicalize(folder)?,
            name
        })
    }

    pub fn load_modules(&self) -> anyhow::Result<Vec<Module>> {
        let mut modules = vec![];

        for entry in fs::read_dir(&self.location)? {
            let entry = entry?.path();

            match Module::try_load(&entry, &self) {
                Ok(Some(m)) => {
                    if modules.iter().any(|n: &Module| n.qualifier == m.qualifier) {
                        warn!("Refused to load {}/'{}', since its qualifier is already taken.", self.name, entry.file_name().map(OsStr::to_string_lossy).unwrap_or(Cow::Borrowed("unknown module")));
                    }

                    modules.push(m);
                }
                Err(e) => {
                    warn!("Failed to load {}/'{}': {e:#}", self.name, entry.file_name().map(OsStr::to_string_lossy).unwrap_or(Cow::Borrowed("unknown module")));
                }
                _ => {}
            }
        }

        Ok(modules)
    }

    /// loads all host files of the repository
    pub fn load_hosts(&self) -> anyhow::Result<Vec<Host>> {
        Ok(
            fs::read_dir(&self.location)?
                .filter_map(|e| e.map(|e| e.path()).ok())
                .filter(|path| path.file_name().map(|name| name.to_string_lossy().ends_with(HOST_CONFIG_FILEENDING)).unwrap_or(false))
                .filter_map(|path| {
                    match Host::try_load(&path, &self) {
                        Ok(host) => Some(host),
                        Err(e) => {
                            warn!("Failed to load host file '{}' of repository {}: {e:#}", path.file_name().map(OsStr::to_string_lossy).unwrap_or(Cow::Borrowed("unknown module")), self.name);
                            None
                        }
                    }
                })
            .collect()
        )
    }

    /// Loads the variables from the repository config
    pub fn load_variables(&self) -> anyhow::Result<Option<Variable>> {
        let path = self.location.join(REPOSITORY_CONFIG);

        let config: RepositoryConfig = serde_yaml::from_reader(
            File::open(&path).with_context(|| format!("failed to open repository config file for {}", &self.name))?
        ).map_err(|e| anyhow!("failed to parse repository config file for {}: {e:#}", &self.name))?;

        Ok(config.variables)
    }
}

/// Validates repository name and insures that it does not mess with the filesystem during caching
fn legal_name(name: &str) -> bool {
    !name.is_empty() && !name.contains('/')
}