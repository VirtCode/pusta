use std::fs::File;
use std::path::PathBuf;
use anyhow::{Context, Result};
use serde::Deserialize;
use crate::module::install::InstallAction;

#[derive(Deserialize)]
pub struct ModuleConfig {
    pub name: String,
    pub description: String,
    pub author: Option<String>,
    pub version: String,

    pub alias: Option<String>,
    pub provides: Option<String>,
    pub depends: Option<String>,

    pub install: Vec<InstallAction>

    // install, actions, variables, lists
}

impl ModuleConfig {
    pub fn load(target: &PathBuf) -> Result<Self> {
        let f = File::open(target).with_context(|| format!("Failed to read module config file '{}', does it exist?", target.to_string_lossy()))?;
        Ok(serde_yaml::from_reader(f)?)
    }
}