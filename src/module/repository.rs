use std::borrow::Cow;
use std::ffi::OsStr;
use std::fs;
use std::fs::File;
use std::path::{Path, PathBuf};
use anyhow::{Context, Error};
use log::warn;
use serde::{Deserialize, Serialize};
use crate::module::Module;

pub const REPOSITORY_CONFIG: &str = "pusta.yml";

#[derive(Deserialize)]
pub struct RepositoryConfig {
    pub alias: Option<String>
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
}