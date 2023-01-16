use std::fs;
use std::fs::File;
use std::path::{Path, PathBuf};
use anyhow::{Context, Error};
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
}