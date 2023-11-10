use std::borrow::Cow;
use std::ffi::OsStr;
use std::fs;
use std::fs::File;
use std::path::{Path, PathBuf};
use anyhow::{anyhow, Context, Error};
use log::warn;
use serde::{Deserialize, Serialize};
use crate::module::Module;
use crate::variables::Variable;

pub const REPOSITORY_CONFIG: &str = "pusta.yml";

#[derive(Deserialize)]
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
        let config: RepositoryConfig = serde_yaml::from_reader(File::open(&path).with_context(|| format!("Failed to open repository config file at {}", path.to_string_lossy()))?).with_context(|| format!("Failed to parse repository config file at {}", path.to_string_lossy()))?;

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
                    warn!("Failed to load {}/'{}': {e}", self.name, entry.file_name().map(OsStr::to_string_lossy).unwrap_or(Cow::Borrowed("unknown module")));
                }
                _ => {}
            }
        }

        Ok(modules)
    }

    /// Loads the variables from the repository config
    pub fn load_variables(&self) -> Option<Variable>{
        let path = self.location.join(REPOSITORY_CONFIG);

        let config: RepositoryConfig = serde_yaml::from_reader(File::open(&path).ok()?).ok()?;
        return config.variables;
    }
}

/// Validates repository name and insures that it does not mess with the filesystem during caching
fn legal_name(name: &str) -> bool {
    !name.is_empty() && !name.contains('/')
}