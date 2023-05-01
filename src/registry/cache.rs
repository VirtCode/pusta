use std::fs;
use std::fs::File;
use std::path::{Path, PathBuf};
use anyhow::{anyhow, Context, format_err};
use log::{debug, error, warn};
use serde::__serialize_unimplemented;
use crate::module::install::InstalledModule;
use crate::module::Module;
use crate::module::qualifier::ModuleQualifier;
use crate::module::repository::Repository;

pub const MODULES: &str = "modules.json";
pub const REPOSITORIES: &str = "repositories.json";
pub const DATA: &str = "data";

/// This struct handles the saving of the installation state of the machine
pub struct Cache {
    folder: PathBuf,
    pub modules: Vec<InstalledModule>,
    pub repositories: Vec<Repository>
}

impl Cache {

    /// Creates a new cache, without loading anything
    pub fn new(path: &Path) -> Self {
        Cache {
            folder: path.to_owned(),
            modules: vec![],
            repositories: vec![]
        }
    }

    /// Loads installed modules and added repositories
    pub fn load(&mut self) {
        self.read_repositories();
        self.read_modules();
    }


    /// Reads installed modules from file
    fn read_modules(&mut self) {
        debug!("Reading installed modules");
        let mut path = self.folder.clone();
        path.push(MODULES);

        self.modules = File::open(path).map_err(|e| anyhow!(e))
            .and_then(|f| serde_json::from_reader(f).context("Failed to deserialize modules"))
            .unwrap_or_else(|e|  {
                warn!("Failed to read installed modules from cache ({}), installed will not be known of", e.to_string());
                vec![]
            });
    }

    /// Writes installed modules to file
    fn write_modules(&self) {
        debug!("Saving installed modules to disk");
        let mut path = self.folder.clone();
        path.push(MODULES);

        File::create(path).map_err(|e| anyhow!(e))
            .and_then(|f| serde_json::to_writer(f, &self.modules).context("failed to serialize modules"))
            .unwrap_or_else(|e| error!("Failed to write module cache ({e}), actions will not be persisted!"));
    }

    /// Adds a module to the installed modules
    pub fn install_module(&mut self, module: InstalledModule) -> anyhow::Result<()> {
        // Sanity check
        if self.modules.iter().any(|m| m.module.qualifier == module.module.qualifier) {
            return Err(anyhow!("The module '{}' was installed twice it seems", module.module.qualifier.unique()))
        }

        self.modules.push(module);
        self.write_modules();

        Ok(())
    }

    /// Removes a module from the installed modules
    pub fn remove_module(&mut self, unique: &str) {
        self.modules.retain(|f| f.module.qualifier.unique() != unique);
        self.write_modules();
    }

    /// Finds an installed module by its qualifier
    pub fn module(&self, unique: &str) -> Option<&InstalledModule> {
        self.modules.iter().find(|m| m.module.qualifier.unique() == unique)
    }

    /// Checks whether it has an installed module
    pub fn has_module(&self, unique: &str) -> bool {
        self.modules.iter().any(|m| m.module.qualifier.unique() == unique)
    }

    /// Returns a possible module with a qualifier
    pub fn find_module(&self, unique: &str) -> Option<&InstalledModule>{
        self.modules.iter().find(|m| { m.module.qualifier.unique() == unique })
    }

    pub fn has_provider(&self, dep: &str) -> bool {
        self.modules.iter().any(|m| m.module.qualifier.does_provide(dep))
    }

    pub fn find_providers(&self, dep: &str) -> Vec<&InstalledModule> {
        self.modules.iter().filter(|m| m.module.qualifier.does_provide(dep)).collect()
    }

    /// Checks everywhere whether a module is being depended on
    pub fn depended_upon(&self, installed: &InstalledModule, exclude: &Vec<ModuleQualifier>) -> Option<&InstalledModule> {
        self.modules.iter().find(|m| {
            m.module.qualifier != installed.module.qualifier && // Avoid modules that depend on themselves
            !exclude.iter().any(|i| *i == m.module.qualifier) && // Ignore excludes
            m.module.dependencies.iter().any(|s| {
                // Concerned module does provide and
                installed.module.qualifier.does_provide(s) &&
                // No-one else does provide
                !self.modules.iter().any(|m| m.module.qualifier != installed.module.qualifier && m.module.qualifier.does_provide(s))
            })
        })
    }

    /// Queries the installed modules for a specific module
    pub fn query_module(&self, query: &str) -> Vec<&InstalledModule>{
        self.modules.iter().filter(|m| {
            if query.contains('/') {
                m.module.qualifier.unique() == query
            } else {
                m.module.qualifier.name() == query
            }
        }).collect()
    }


    /// Reads added repositories from file
    fn read_repositories(&mut self) {
        debug!("Reading added source repositories");
        let mut path = self.folder.clone();
        path.push(REPOSITORIES);

        self.repositories = File::open(path).map_err(|e| anyhow!(e))
            .and_then(|f| serde_json::from_reader(f).context("Failed to deserialize repositories"))
            .unwrap_or_else(|e|  {
                warn!("Failed to read added repositories from cache ({}), no repositories will be available", e.to_string());
                vec![]
            });
    }

    /// Writes added repositories to file
    fn write_repositories(&self) {
        debug!("Saving added source repositories to disk");
        let mut path = self.folder.clone();
        path.push(REPOSITORIES);

        File::create(path).map_err(|e| anyhow!(e))
            .and_then(|f| serde_json::to_writer(f, &self.repositories).context("Failed to serialize repositories"))
            .unwrap_or_else(|e| error!("Failed to write repository cache ({e}), actions will not be persisted!"));
    }

    /// Adds a repository to the added sources
    pub fn add_repository(&mut self, repo: Repository) -> anyhow::Result<()>{

        // Check if any other repo already exists
        if self.repositories.iter().any(|r| r.name == repo.name) {
            return Err(anyhow!("There is already a repository loaded with the same alias '{}'", &repo.name))
        }
        if let Some(r) = self.repositories.iter().find(|r| r.location == repo.location) {
            return Err(anyhow!("This repository is already added (under the alias '{}')", &r.name))
        }

        self.repositories.push(repo);
        self.write_repositories();

        Ok(())
    }

    /// Removes a repositories from the added sources
    pub fn remove_repository(&mut self, name: &str) -> Option<Repository> {
        let result = self.repositories.iter().position(|r| r.name == name).map(|i| self.repositories.swap_remove(i));

        if result.is_some() {
            self.write_repositories();
        }
        
        result
    }

    /// Finds an added repository by its name
    pub fn repository(&self, name: &str) -> Option<&Repository> {
        self.repositories.iter().find(|r| r.name == name)
    }


    /// Creates a module cache folder for the job cache of the modules
    pub fn create_module_cache(&self, module: &Module) -> anyhow::Result<PathBuf> {
        let mut path = self.folder.clone();
        path.push(DATA);
        path.push(module.qualifier.unique().replace('/', "-"));

        fs::create_dir_all(&path)?;

        Ok(path)
    }

    /// Migrates a module cache from an old one to a new one
    pub fn migrate_module_cache(&self, old_module: &Module, new_module: &Module) -> anyhow::Result<()> {

        let mut old = self.folder.clone();
        old.push(DATA);
        old.push(old_module.qualifier.unique().replace('/', "-"));

        let mut new = self.folder.clone();
        new.push(DATA);
        new.push(new_module.qualifier.unique().replace('/', "-"));

        fs::rename(&old, &new)?;
        Ok(())
    }

    /// Removes a module cache folder
    pub fn delete_module_cache(&self, module: &Module) -> anyhow::Result<()> {
        let mut path = self.folder.clone();
        path.push(DATA);
        path.push(module.qualifier.unique().replace('/', "-"));

        // For some reason, this function works but always returns an error, this is why it is ignored here
        // TODO: Do more investigations regarding fs::remove_dir_all
        let _ = fs::remove_dir_all(&path);
        Ok(())
    }
}