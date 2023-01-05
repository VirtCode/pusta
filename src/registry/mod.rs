use std::collections::HashMap;
use std::fs::File;
use std::path::PathBuf;
use anyhow::{Context, Error};
use log::error;
use crate::config::Config;
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

    pub fn write_modules(&self) {
        File::create(crate::CACHE_MODULES).context("Failed to open module cache file")
            .and_then(|f| serde_json::to_writer(f, &self.installed).context("Failed to serialize modules"))
            .unwrap_or_else(|e| error!("Failed to write module cache ({e}), actions will not be persisted!"));
    }

    pub fn write_repositories(&self) {
        File::create(crate::CACHE_REPOSITORIES).context("Failed to open repository cache file")
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
        if !name.contains('/') && self.main.is_some() {
            let main = &self.main.expect("This cannot be none!");

            // Could probably be optimizable... but my rust knowledge is too limited and i don't wanna have to use a mutable iterator
            let mains: Vec<String> = candidates.filter(|q| q.repository() == main).map(|q| q.unique().clone()).collect();
            if let Some(m) = mains.first() { return vec!(m.clone()) }
        }

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

    pub fn add(&mut self, path: &PathBuf, alias: Option<&String>) -> anyhow::Result<&Repository>{
        let repo = Repository::load(path, alias)?;

        // Check repository conflicts
        if self.repositories.contains_key(&repo.name) {
            return Err(Error::msg(format!("There is already a repository loaded with the same alias '{}'", repo.name)))
        }

        if let Some((_, r)) = self.repositories.iter().find(|(_, r)| { r.location == repo.location }) {
            return Err(Error::msg(format!("This repository is already added (under the alias '{}')", &r.name)))
        }

        // Check module conflicts
        if let Some(qualifier) = repo.check_qualifier_conflicts() {
            return Err(Error::msg(format!("Two or more modules qualify for the qualifier '{}'", qualifier)))
        }

        let name = repo.name.clone();
        self.repositories.insert(repo.name.clone(), repo);

        Ok(self.repositories.get(&name).unwrap())
    }

    pub fn remove(&mut self, alias: &str) {
        self.repositories.remove(alias);
    }

    pub fn get_repository(&self, repo: &str) -> Option<&Repository> {

        let repo = if let Some(main) = &self.main {
            main.as_str()
        } else {
            repo
        };

        self.repositories.get(repo)
    }

    pub fn get_from(&self, repo: &str, qualifier: &str) -> Option<&Module> {
        let repo = if repo == MAIN && self.main.is_some() {
            self.main.clone().unwrap()
        } else { repo.to_string() };

        self.repositories.get(&repo).and_then(|r| r.module(qualifier))
    }

    pub fn provider(&self, qualifier: &str) -> Vec<&Module> {
        let mut vec = vec![];

        for (_, repo) in &self.repositories {
            vec.append(&mut repo.provider(qualifier))
        }

        vec
    }
}