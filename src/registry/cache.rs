use std::{env, fs};
use std::env::VarError;
use std::fs::File;
use std::path::{Path, PathBuf};
use anyhow::{anyhow, Context};
use log::{debug, error, info, warn};
use crate::config::Config;
use crate::module::install::InstalledModule;
use crate::module::Module;
use crate::module::qualifier::ModuleQualifier;
use crate::module::repository::Repository;
use crate::registry::index::Index;

pub const DEFAULT_PARENT: &str = "~/.local/state";
pub const DEFAULT_DIR: &str = "/pusta";

pub const MODULES: &str = "modules.json";
pub const REPOSITORIES: &str = "repositories.json";
pub const DATA: &str = "data";

/// Finds the current default cache directory (XDG_STATE_HOME)
pub fn default_cache_dir() -> String {
    let parent = match env::var("XDG_STATE_HOME") {
        Ok(s) => { s }
        Err(_) => { DEFAULT_PARENT.to_owned() }
    };

    parent + DEFAULT_DIR
}

/// This struct handles the saving of the installation state of the machine
pub struct Cache {
    folder: PathBuf,
    pub index: Index<InstalledModule>,
    pub repositories: Vec<Repository>
}

impl Cache {

    /// Creates a new cache, without loading anything
    pub fn new(config: &Config) -> Self {
        Cache {
            folder: PathBuf::from(shellexpand::tilde(&config.cache_dir).to_string()),
            index: Index::new(),
            repositories: vec![]
        }
    }

    /// Loads installed modules and added repositories
    pub fn load(&mut self) -> anyhow::Result<()>{
        self.read_repositories()?;
        self.read_modules()
    }


    /// Reads installed modules from file
    fn read_modules(&mut self) -> anyhow::Result<()> {
        debug!("Reading installed modules");
        let mut path = self.folder.clone();
        path.push(MODULES);

        if !path.exists() {
            info!("Module cache file does not exist, saving empty state...");
            fs::create_dir_all(&self.folder)?;
            self.write_modules()?;
        }

        self.index.modules = File::open(path).map_err(|e| anyhow!(e))
            .and_then(|f| serde_json::from_reader(f).context("Failed to deserialize modules"))
            .context("Failed to read installed modules")?;

        Ok(())
    }

    /// Writes installed modules to file
    fn write_modules(&self) -> anyhow::Result<()> {
        debug!("Saving installed modules to disk");
        let mut path = self.folder.clone();
        path.push(MODULES);

        File::create(path).map_err(|e| anyhow!(e))
            .and_then(|f| serde_json::to_writer(f, &self.index.modules).context("Failed to serialize modules"))
            .context("Failed to save installed modules, module changes will not be persisted")
    }

    /// Reads added repositories from file
    fn read_repositories(&mut self) -> anyhow::Result<()>{
        debug!("Reading added source repositories");
        let mut path = self.folder.clone();
        path.push(REPOSITORIES);

        if !path.exists() {
            info!("Repository cache file does not exist, saving empty state...");
            fs::create_dir_all(&self.folder)?;
            self.write_repositories()?;
        }

        self.repositories = File::open(path).map_err(|e| anyhow!(e))
            .and_then(|f| serde_json::from_reader(f).context("Failed to deserialize repositories"))
            .context("Failed to read added repositories")?;

        Ok(())
    }

    /// Writes added repositories to file
    fn write_repositories(&self) -> anyhow::Result<()>{
        debug!("Saving added source repositories to disk");
        let mut path = self.folder.clone();
        path.push(REPOSITORIES);

        File::create(path).map_err(|e| anyhow!(e))
            .and_then(|f| serde_json::to_writer(f, &self.repositories).context("Failed to serialize repositories"))
            .context("Failed to save added repositories, repository changes will not be persisted")
    }


    /// Adds a module to the installed modules
    pub fn install_module(&mut self, module: InstalledModule) -> anyhow::Result<()> {
        self.index.add(module);
        self.write_modules()
    }

    /// Removes a module from the installed modules
    pub fn remove_module(&mut self, qualifier: &ModuleQualifier) -> anyhow::Result<()> {
        self.index.remove(qualifier);
        self.write_modules()
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
        self.write_repositories()
    }

    /// Removes a repositories from the added sources
    pub fn remove_repository(&mut self, name: &str) -> anyhow::Result<Option<Repository>> {
        let result = self.repositories.iter()
            .position(|r| r.name == name)
            .map(|i| self.repositories.swap_remove(i));

        if result.is_some() {
            self.write_repositories()?;
        }
        
        Ok(result)
    }

    /// Finds an added repository by its name
    pub fn get_repository(&self, name: &str) -> Option<&Repository> {
        self.repositories.iter().find(|r| r.name == name)
    }


    /// Creates a module cache folder for the job cache of the modules
    pub fn create_module_cache(&self, module: &Module) -> anyhow::Result<PathBuf> {
        let mut path = self.folder.clone();
        path.push(DATA);
        path.push(module.qualifier.unique());

        fs::create_dir_all(&path)?;

        Ok(path)
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