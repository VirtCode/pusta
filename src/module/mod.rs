use std::fs;
use std::path::PathBuf;
use anyhow::Error;
use serde::{Deserialize, Serialize};
use crate::module::reader::ModuleConfig;

mod reader;

const MODULE_CONFIG: &str = "module.yml";

#[derive(Serialize, Deserialize)]
pub struct Module {

    path: PathBuf,
    pub qualifier: ModuleQualifier,

    name: String,
    description: String,
    version: String

}

impl Module {
    pub fn create(dir: &PathBuf) -> anyhow::Result<Option<Self>> {
        // Module has to be directory
        if !dir.is_dir() { return Ok(None) }

        // Module has to contain a module.yml file
        let mut config = dir.clone();
        config.push(MODULE_CONFIG);
        if !config.exists() || !config.is_file() { return Ok(None) }

        // Read config file
        let config = ModuleConfig::load(&config)?;

        Ok(Some(Self {
            path: dir.clone(),

            qualifier: ModuleQualifier {
                provide: config.provides,
                alias: config.alias,
                dir: dir.file_name().ok_or_else(|| Error::msg("Failed to get module directory name!"))?.to_string_lossy().to_string()
            },

            name: config.name,
            description: config.description,
            version: config.version
        }))
    }
}

#[derive(Serialize, Deserialize)]
pub struct ModuleQualifier {
    dir: String,
    alias: Option<String>,
    provide: Option<String>
}

impl ModuleQualifier {

    pub fn qualifies(&self, qualifier: &str) -> bool {

        // Provides module
        if let Some(provide) = &self.provide {
            if provide == qualifier { return true }
        }

        // Is the module
        if self.name() == qualifier { return true }

        false
    }

    pub fn name(&self) -> &String {
        if let Some(alias) = &self.alias {
            alias
        } else {
            &self.dir
        }
    }


    pub fn provides(&self) -> &Option<String> {
        &self.provide
    }
}

