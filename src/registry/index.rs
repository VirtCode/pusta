use std::fs;
use anyhow::Error;
use log::{debug, warn};
use crate::module::Module;
use crate::module::repository::Repository;

pub struct Index {
    modules: Vec<Module>
}

impl Index {
    
    /// Initializes a new Index
    pub fn new() -> Self {
        Index { modules: vec![] }
    }

    /// Loads modules from a given repository
    pub fn load_repository(&mut self, repo: &Repository) -> anyhow::Result<()>{
        debug!("Loading modules from source repository '{}'", &repo.name);

        let mut modules: Vec<Module> = vec![];

        for entry in fs::read_dir(&repo.location)? {
            let entry = entry?.path();

            // Is folder
            if entry.is_dir() {
                let mut file = entry.clone();
                file.push(crate::FILE_MODULE);

                // Has module.yml
                if file.exists() {

                    debug!("Loading module at '{}'", entry.to_string_lossy());
                    match Module::load(&entry, repo) {

                        Ok(module) => {
                            if modules.iter().any(|m| m.qualifier.name() == module.qualifier.name()) { return Err(Error::msg(format!("Failed to load modules, name conflict found ('{}')", module.qualifier.name()))) }

                            modules.push(module);
                        },
                        Err(e) => {
                            warn!("Failed to load module (from '{}' at '{}'): {}", repo.name, entry.file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_default(), e.to_string());
                        }
                    }
                }
            }
        }

        // Pass over modules to index
        self.modules.append(&mut modules);

        Ok(())
    }

    /// Unloads all modules belonging to an added repository
    pub fn unload_repository(&mut self, repo: &Repository) {
        self.modules.retain(|r| *r.qualifier.repository() != repo.name);
    }

    /// Loads modules from all given repositories
    pub fn load_repositories(&mut self, repos: &Vec<Repository>) -> anyhow::Result<()>{
        for x in repos {
            self.load_repository(x)?;
        }

        Ok(())
    }

    /// Queries the index for one or more modules, based on the query parameter
    pub fn query(&self, query: &str) -> Vec<&Module> {
        self.modules.iter()
            .filter(|m| {
                if query.contains('/') {
                    m.qualifier.unique() == query
                } else {
                    m.qualifier.name() == query
                }
            })
            .collect()
    }

    /// Queries the index for modules providing the given provider
    pub fn providers(&self, provider: &str) -> Vec<&Module> {
        self.modules.iter()
            .filter(|m| m.qualifier.does_provide(provider))
            .collect()
    }
}