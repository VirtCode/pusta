use std::collections::HashMap;
use std::fs;
use std::fs::File;
use std::path::PathBuf;
use anyhow::{Context, Error, format_err};
use log::{error, warn};
use crate::config::Config;
use crate::FILE_MODULE;
use crate::module::install::InstalledModule;
use crate::module::Module;
use crate::module::repository::Repository;

pub const MAIN: &str = "main";

// TODO: Make new Registry, Loading all modules into one Vec (ofc, doing duplicate check before), also loading installed modules
// Add methods to extract one module owning, so it can be installed
// Also add providing conflict detection
// Only use repositories for loading modules, not as a struct inbetween
// Query method -> Vec<Strings> (names of matching modules), peek method (String) -> &Module, retrieve method (String) -> Module
// Rewrite cache too

// NEW TODO: Add methods for adding and removing repos, load repos and modules from cache, do duplicate detection

// This code is already garbage, please revamp the registry in the future
pub struct Registry {
    strict: bool,

    main: Option<String>,
    available: Vec<Module>,
    installed: Vec<InstalledModule>,
    repositories: Vec<Repository>
}

impl Registry {
    pub fn new(config: &Config) -> Self {
        Registry {
            strict: config.repositories.strict_qualifying,
            main: config.repositories.main.clone(),
            available: vec![],
            installed: vec![],
            repositories: vec![]
        }
    }

    pub fn read_modules(&mut self) {
        let path = shellexpand::tilde(crate::CACHE_MODULES).to_string();

        self.installed = File::open(path).map_err(|e| format_err!(e))
            .and_then(|f| serde_json::from_reader(f).context("Failed to deserialize modules"))
            .unwrap_or_else(|e|  {
                warn!("Failed to read installed modules from cache ({}), installed will not be known of", e.to_string());
                vec![]
            });
    }

    pub fn read_repositories(&mut self) -> anyhow::Result<()> {
        let path = shellexpand::tilde(crate::CACHE_REPOSITORIES).to_string();

        let repositories: Vec<Repository> = File::open(path).map_err(|e| format_err!(e))
            .and_then(|f| serde_json::from_reader(f).context("Failed to deserialize repositories"))
            .unwrap_or_else(|e|  {
                warn!("Failed to read added repositories from cache ({}), no repositories will be available", e.to_string());
                vec![]
            });

        for x in &repositories {
            self.load_modules(x)?;
        }

        self.repositories = repositories;

        Ok(())
    }

    fn write_modules(&self) {
        let path = shellexpand::tilde(crate::CACHE_MODULES).to_string();

        File::create(path).map_err(|e| format_err!(e))
            .and_then(|f| serde_json::to_writer(f, &self.installed).context("failed to serialize modules"))
            .unwrap_or_else(|e| error!("Failed to write module cache ({e}), actions will not be persisted!"));
    }

    fn write_repositories(&self) {
        let path = shellexpand::tilde(crate::CACHE_REPOSITORIES).to_string();

        File::create(path).map_err(|e| format_err!(e))
            .and_then(|f| serde_json::to_writer(f, &self.repositories).context("Failed to serialize repositories"))
            .unwrap_or_else(|e| error!("Failed to write repository cache ({e}), actions will not be persisted!"));
    }

    pub fn query(&self, name: &str) -> Vec<String> {
        // Get every matching thing
        let candidates = self.available.iter().chain(self.installed.iter().map(|m| &m.module)).map(|m| &m.qualifier)
            .filter(|q| {
                if name.contains('/') {
                    q.unique() == name
                } else {
                    q.name() == name
                }
            });

        // Prefer main if there is one
        // if !name.contains('/') && self.main.is_some() {
        //     let main = self.main.as_ref().expect("This cannot be none!");
        //
        //     // Could probably be optimizable... but my rust knowledge is too limited and i don't wanna have to use a mutable iterator
        //     let mains: Vec<String> = candidates.filter(|q| q.repository() == main).map(|q| q.unique().clone()).collect();
        //     if let Some(m) = mains.first() { return vec!(m.clone()) }
        // }

        let mut result: Vec<String> = candidates.map(|q| q.unique().clone()).collect();
        result.sort();
        result.dedup();

        result
    }

    pub fn get(&self, qualifier: &str) -> Option<&Module> {
        self.available.iter().filter(|m| m.qualifier.unique() == qualifier).collect::<Vec<&Module>>().first().map(|m| *m)
    }

    pub fn get_installed(&self, qualifier: &str) -> Option<&InstalledModule> {
        self.installed.iter().filter(|m| m.module.qualifier.unique() == qualifier).collect::<Vec<&InstalledModule>>().first().map(|m| *m)
    }

    pub fn install(&mut self, module: InstalledModule) {
        self.installed.push(module);

        self.write_modules();
    }

    pub fn uninstall(&mut self, module: &InstalledModule) {
        if let Some(i) = self.installed.iter().position(|m| module.module.qualifier.unique() == m.module.qualifier.unique()) {
            self.installed.remove(i);
        }

        self.write_modules();
    }

    pub fn add(&mut self, path: &PathBuf, alias: Option<&str>) -> anyhow::Result<()>{
        let repo = Repository::load(path, alias)?;

        // Check if any other repo already exists
        if self.repositories.iter().any(|r| r.name == repo.name) {
            return Err(Error::msg(format!("There is already a repository loaded with the same alias '{}'", &repo.name)))
        }
        if let Some(r) = self.repositories.iter().find(|r| r.location == repo.location) {
            return Err(Error::msg(format!("This repository is already added (under the alias '{}')", &r.name)))
        }

        // Add repositories
        self.load_modules(&repo)?;
        self.repositories.push(repo);
        self.write_repositories();


        Ok(())
    }

    pub fn unadd(&mut self, alias: &str) -> bool {
        if let Some(i) = self.repositories.iter().position(|r| r.name == alias) {
            let remains = self.repositories.remove(i);
            self.write_repositories();

            // Remove available modules
            self.available.retain(|r| *r.qualifier.repository() != remains.name);

            true
        } else { false }
    }

    fn load_modules(&mut self, repo: &Repository) -> anyhow::Result<()> {

        let mut modules: Vec<Module> = vec![];

        for entry in fs::read_dir(&repo.location)? {
            let entry = entry?.path();

            // Is folder
            if entry.is_dir() {
                let mut file = entry.clone();
                file.push(FILE_MODULE);

                // Has module.yml
                if file.exists() {

                    match Module::load(&entry, repo) {
                        Ok(module) => {
                            if modules.iter().any(|m| m.qualifier.name() == module.qualifier.name()) { return Err(Error::msg(format!("Failed to load modules, name conflict found ('{}')", module.qualifier.name())))}

                            modules.push(module);
                        },
                        Err(e) => {
                            error!("Failed to load module (from '{}' at '{}'): {}", repo.name, entry.file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_default(), e.to_string());
                        }
                    }
                }
            }
        }

        self.available.append(&mut modules);

        Ok(())
    }
}