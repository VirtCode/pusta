use std::fs;
use anyhow::Error;
use log::{debug, warn};
use crate::module::install::InstalledModule;
use crate::module::Module;
use crate::module::qualifier::ModuleQualifier;
use crate::module::repository::Repository;

pub trait Indexable {
    fn dependencies(&self) -> &Vec<String>;
    fn qualifier(&self) -> &ModuleQualifier;
}

pub struct Index<T> where T: Indexable {
    pub modules: Vec<T>
}

impl<T> Index<T> where T: Indexable {
    
    /// Creates a new index
    pub fn new() -> Self {
        Self { modules: vec![] }
    }

    /// Returns a list of modules which are matched by a given query
    pub fn query(&self, query: &str) -> Vec<&T> {

        self.modules.iter()
            .filter(|m| {
                if query.contains('/') {
                    m.qualifier().unique() == query
                } else {
                    m.qualifier().name() == query
                }
            })
            .collect()

    }

    /// Returns a list of modules which provide a certain dependency
    pub fn providers(&self, dependency: &str) -> Vec<&T> {
        self.modules.iter()
            .filter(|m| m.qualifier().does_provide(dependency))
            .collect()
    }

    /// Returns a list of modules which may depend on a given module
    pub fn dependents(&self, dependency: &ModuleQualifier) -> Vec<&T> {
        self.modules.iter()
            .filter(|m| {
                // Avoid modules that depend on themselves
                m.qualifier() != dependency &&

                m.dependencies().iter().any(|s| dependency.does_provide(s))
            })
            .collect()
    }

    /// Returns a module for the given qualifier
    pub fn get(&self, qualifier: &ModuleQualifier) -> Option<&T> {
        self.modules.iter().find(|m| m.qualifier() == qualifier)
    }

    /// Adds a module or replaces a given one if needed
    pub fn add(&mut self, module: T) {
        // Remove possible duplicates
        self.remove(module.qualifier());

        self.modules.push(module);
    }
    
    /// Removes a module from the index if present
    pub fn remove(&mut self, qualifier: &ModuleQualifier) {
        self.modules.retain(|f| f.qualifier() != qualifier);
    }
}

impl Index<Module> {
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
}

impl Index<InstalledModule> {
    
}