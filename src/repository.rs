use std::fs;
use std::path::{Path, PathBuf};
use anyhow::Error;
use serde::{Deserialize, Serialize};
use crate::module::Module;


#[derive(Serialize, Deserialize)]
pub struct Repository {

    location: PathBuf,
    pub name: String,

    modules: Vec<Module>,

}

impl Repository {
    pub fn load(folder: &PathBuf) -> anyhow::Result<Self>{

        let name = folder.file_name().ok_or_else(|| Error::msg("Failed to get repo dir name"))?.to_string_lossy().to_string();

        let mut modules = vec![];

        // Load modules
        for x in fs::read_dir(folder)? {
            let file = x?.path();

            if file.is_dir() {
                let module = Module::create(&name, &file)?;

                if let Some(module) = module { modules.push(module) }
            }
        }

        Ok(Repository {
            location: folder.clone(),
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