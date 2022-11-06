use std::fs;
use std::fs::File;
use std::path::{Path, PathBuf};
use anyhow::{Context, Error};
use serde::{Deserialize, Serialize};
use crate::module::Module;

pub const REPOSITORY_CONFIG: &str = "pusta.yml";


#[derive(Serialize, Deserialize)]
pub struct Repository {

    pub location: PathBuf,
    pub name: String,

    modules: Vec<Module>,

}

impl Repository {
    pub fn load(folder: &PathBuf, alias: Option<&String>) -> anyhow::Result<Self>{

        // Read config
        let path = folder.clone().join(REPOSITORY_CONFIG);
        let config: RepoConfig = serde_yaml::from_reader(File::open(folder.clone().join(REPOSITORY_CONFIG)).with_context(|| format!("Failed to open repository config file at {}", path.to_string_lossy()))?).with_context(|| format!("Failed to parse repository config file at {}", path.to_string_lossy()))?;

        let name = if let Some(alias) = alias {
            alias.clone()
        } else if let Some(alias) = config.repository.alias {
            alias
        } else {
            folder.file_name().ok_or_else(|| Error::msg("Failed to get repo dir name"))?.to_string_lossy().to_string()
        };


        // Load modules
        let mut modules = vec![];

        for x in fs::read_dir(folder)? {
            let file = x?.path();

            if file.is_dir() {
                let module = Module::create(&name, &file)?;

                if let Some(module) = module { modules.push(module) }
            }
        }

        Ok(Repository {
            location: fs::canonicalize(folder)?,
            name,
            modules
        })
    }

    pub fn check_qualifier_conflicts(&self) -> Option<&String> {

        let mut names = vec![];
        let mut provides = vec![];

        for module in &self.modules {
            if names.contains(module.qualifier.name()) || provides.contains(module.qualifier.name()) {
                return Some(module.qualifier.name());
            }

            if let Some(prov) = module.qualifier.provide() {
                if names.contains(prov) { return Some(prov) }

                provides.push(prov.clone());
            }

            names.push(module.qualifier.name().clone());
        }

        None
    }

    pub fn module(&self, qualifier: &str) -> Option<&Module> {
        self.modules.iter().find(|m| m.qualifier.is(qualifier))
    }

    pub fn provider(&self, qualifier: &str) -> Vec<&Module> {
        self.modules.iter().filter(|m| {
            m.qualifier.does_provide(qualifier)
        }).collect()
    }
}

#[derive(Serialize, Deserialize)]
struct RepoConfig {
    repository: RepoData
}

#[derive(Serialize, Deserialize)]
struct RepoData {
    alias: Option<String>
}