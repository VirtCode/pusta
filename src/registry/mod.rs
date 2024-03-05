pub mod index;
pub mod cache;

use std::ops::Deref;
use std::path::{Path, PathBuf};
use chrono::{DateTime, Local, NaiveDateTime};
use colored::Colorize;
use log::{debug, error, info, warn};
use crate::config::Config;
use crate::module::install::{Gatherer, InstalledModule, modify};
use crate::module::Module;
use crate::module::qualifier::ModuleQualifier;
use crate::module::repository::Repository;
use crate::output::{logger, prompt_choice_module, prompt_yn};
use crate::output::logger::{disable_indent, enable_indent, section};
use crate::registry::cache::Cache;
use crate::registry::index::{Index, Indexable};
use crate::variables::{generate_magic, load_system, Variable};

/// This struct handles all modules and modifies them. Essentially, every change in install state goes through this struct.
pub struct Registry {
    index: Index<Module>,
    cache: Cache,
    config: Config
}

impl Registry {

    /// Creates a new registry
    pub fn new(config: &Config) -> Self {
        Registry {
            index: Index::new(),
            cache: Cache::new(&config),
            config: (*config).clone()
        }
    }

    /// Loads cache and indexes all current modules
    pub fn load(&mut self) -> anyhow::Result<()> {
        self.cache.load()?;

        // Index modules
        for repo in &self.cache.repositories {
            match repo.load_modules() {
                Ok(vec) => { self.index.add_all(vec) }
                Err(e) => { warn!("Failed to index repository '{}': {e}", repo.name) }
            }
        }

        Ok(())
    }

    /// Adds a repository
    pub fn add_repository(&mut self, repository: &Path, alias: Option<&str>) {
        info!("Adding repository at '{}' to sources{}...",
                        repository.canonicalize().unwrap().to_string_lossy(),
                        alias.as_ref().map(|s| format!(" (under custom alias '{s}')")).unwrap_or_default());

        let repository = match Repository::load(repository, alias)  {
            Ok(r) => r,
            Err(e) => {
                error!("Failed to load specified repository ({}), does it exist?", e.to_string());
                return;
            }
        };

        let name = repository.name.clone();
        if let Err(e) = self.cache.add_repository(repository) {
            error!("Couldn't add repository: {}", e.to_string());
            return;
        }

        info!("Loading modules from the repository...");
        println!();

        let repository = self.cache.get_repository(&name).expect("just added repo is no longer present?!?");
        match repository.load_modules() {
            Ok(modules) => { self.index.add_all(modules); }
            Err(e) => { warn!("Failed to load modules from this repository: {e}"); }
        }

        info!("Successfully added repository")
    }

    /// Removes a repository
    pub fn remove_repository(&mut self, name: &str) {
        info!("Removing source repository under alias '{name}'");

        match self.cache.remove_repository(name) {
            Ok(Some(repo)) => {
                self.index.remove_repository(&repo);
                info!("Successfully removed and unloaded repository")
            }
            Ok(None) => { error!("There is no repository added under the alias '{name}'") }
            Err(e) => { error!("Failed to remove repository: {e}") }
        }
    }

    /// Installs a module to the system
    pub fn install_module(&mut self, name: &str) {
        section("Querying sources...");
        let modules = self.index.query(name);

        let module = if let Some(m) =
            prompt_choice_module(&modules, "Which module do you mean?")
                .and_then(|i| modules.get(i).map(|m| m.qualifier.clone())) { m } else {

            error!("Couldn't find a module under this name, are relevant sources added?");
            return;
        };

        if self.cache.index.get(&module).is_some() {
            error!("Module is already installed, please update or reinstall it");
            return;
        }

        let mut gatherer = Gatherer::default();

        if let Err(e) = gatherer.install(module, &self.cache.index, &self.index) {
            error!("{e}");
            return;
        }

        debug!("Starting modify");
        modify(gatherer, &self.index, &mut self.cache, &self.config);
    }

    /// Uninstalls a module from the system
    pub fn uninstall_module(&mut self, name: &str) {
        section("Querying cache...");
        let modules = self.cache.index.query(name);

        let module = if let Some(m) = prompt_choice_module(
            &modules.iter().map(|i| &i.module).collect(),
            "Which module do you want to remove?")
            .and_then(|i| modules.get(i).map(|m| m.qualifier().clone())) { m } else {

            error!("Couldn't find installed module for '{name}', is it installed?");
            return;
        };

        let mut gatherer = Gatherer::default();

        if let Err(e) = gatherer.remove(module, &self.cache.index, &self.index) {
            error!("{e}");
            return;
        }

        debug!("Starting modify");
        modify(gatherer, &self.index, &mut self.cache, &self.config);
    }

    /// Updates all modules
    pub fn update_everything(&mut self) {
        section("Looking for updates...");
        let magic = generate_magic();
        let system = load_system(&self.config).unwrap_or(Variable::base());

        let updatable: Vec<ModuleQualifier> = self.cache.index.modules.iter().filter_map(|installed| {

            if let Some(indexed) = self.index.get(installed.qualifier()) {
                if !installed.up_to_date(indexed, &magic, &system, &self.cache) {
                    info!("Found outdated module {}", installed.qualifier().unique());
                    return Some(installed.qualifier().clone())
                }
            }

            None
        }).collect();

        let mut gatherer = Gatherer::default();
        for q in updatable {
            if let Err(e) = gatherer.update(q, &self.cache.index, &self.index) {
                error!("{e}");
                return;
            }
        }

        debug!("Starting modify");
        modify(gatherer, &self.index, &mut self.cache, &self.config);
    }

    /// Updates a single module
    pub fn update_module(&mut self, name: &str) {
        section("Querying cache...");

        let modules = self.cache.index.query(name);

        let module = if let Some(m) = prompt_choice_module(
            &modules.iter().map(|i| &i.module).collect(),
            "Which module do you want to update?")
            .and_then(|i| modules.get(i).map(|m| m.qualifier().clone())) { m } else {

            error!("No module under the name '{name}' is installed, try installing one first");
            return;
        };

        if self.index.get(&module).is_none() {
            error!("Module is installed but is orphaned, so it cannot be updated");
            return;
        }

        // TODO: Check if outdated first

        let mut gatherer = Gatherer::default();
        if let Err(e) = gatherer.update(module, &self.cache.index, &self.index) {
            error!("{e}");
            return;
        }

        debug!("Starting modify");
        modify(gatherer, &self.index, &mut self.cache, &self.config);
    }

    /// Lists modules and repositories
    pub fn list(&self) {
        info!("Added source repositories:");
        enable_indent();

        if self.cache.repositories.is_empty() {
            info!("{}", "No sources are currently added".italic().dimmed())
        } else {
            for repo in &self.cache.repositories {
                info!("{} ({})",
                    repo.name.bold(),
                    repo.location.to_string_lossy())
            }
        }

        disable_indent();
        println!();

        info!("Installed modules:");
        enable_indent();

        if self.cache.index.modules.is_empty() {
            info!("{}", "No modules are currently installed".italic().dimmed())
        } else {
            let magic = generate_magic();
            let system = load_system(&self.config).unwrap_or(Variable::base());

            for module in &self.cache.index.modules {
                let naive: DateTime<Local> = module.built.time.into();

                let info = if let Some(indexed) = self.index.get(&module.module.qualifier) {
                    if !module.up_to_date(&indexed, &magic, &system, &self.cache) {
                        format!("-{}", "outdated".yellow())
                    } else {
                        String::default()
                    }
                } else {
                    format!("-{}", "orphaned".red())
                };

                info!("{} ({}-{}{}) {} {}",
                    module.module.name.bold(),
                    module.module.qualifier.unique(),
                    module.module.version.dimmed(),
                    info,
                    "on".italic(),
                    naive.format("%x").to_string().italic());
            }
        }
        disable_indent();
        println!();
    }

    /// Queries for modules
    pub fn query_module(&self, query: &str) {
        let modules = self.index.query(query);

        if modules.is_empty() {
            info!("{}", "No modules qualify for this query".dimmed().italic())
        } else {
            for module in modules {
                let installed = if !self.cache.index.get(&module.qualifier).is_none() {
                    "installed"
                } else { "" };

                let author = module.author.as_ref().map(|a| format!("by {a}")).unwrap_or_default();

                info!("{}-{} {}\n {} {}\n {}\n",
                    module.qualifier.unique(),
                    module.version.dimmed(),
                    installed.blue(),
                    &module.name.bold(),
                    author,
                    module.description
                )
            }
        }
    }
}