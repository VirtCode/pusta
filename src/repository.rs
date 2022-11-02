use std::fs;
use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};
use crate::module::Module;

#[derive(Serialize, Deserialize)]
struct Respository {

    location: PathBuf,

    modules: Vec<Module>,

}

impl Respository {
    pub fn load(folder: &PathBuf) -> anyhow::Result<Self>{

        let mut modules = vec![];

        // Load modules
        for x in fs::read_dir(folder)? {
            let file = x?.path();

            if file.is_dir() {
                let module = Module::create(&file)?;

                if let Some(module) = module { modules.push(module) }
            }
        }


        Ok(Respository {
            location: folder.clone(),
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

            if let Some(prov) = module.qualifier.provides() {
                if names.contains(prov) { return Some(prov) }

                provides.push(prov.clone());
            }

            names.push(module.qualifier.name().clone());
        }

        None
    }
}