pub mod index;
pub mod cache;
mod transaction;
mod depend;

use std::ops::Deref;
use std::os::unix::raw::time_t;
use std::path::{Path, PathBuf};
use anyhow::{Error, format_err};
use chrono::{DateTime, Local, NaiveDateTime};
use colored::Colorize;
use log::{debug, error, info, warn};
use crate::config::Config;
use crate::module::install::checked::CheckedShell;
use crate::module::install::{InstalledModule, Installer, InstallReason};
use crate::module::install::InstallReason::{Dependency, Manual};
use crate::module::install::shell::Shell;
use crate::module::Module;
use crate::module::qualifier::ModuleQualifier;
use crate::module::repository::Repository;
use crate::output;
use crate::output::{logger, prompt_choice_module, prompt_yn};
use crate::output::logger::{disable_indent, enable_indent};
use crate::registry::cache::Cache;
use crate::registry::depend::DependencyResolver;
use crate::registry::index::{Index, Indexable};
use crate::registry::transaction::ModuleTransaction;

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
                        alias.as_ref().map(|s| format!(" (under alias '{s}')")).unwrap_or_default());

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
        info!("Querying sources...");
        let modules = self.index.query(name);

        let module = if let Some(m) =
            prompt_choice_module(&modules, "Which module do you mean?")
                .and_then(|i| modules.get(i).copied()) { m } else {

            error!("Couldn't find a module under this name, are relevant sources added?");
            return;
        };

        let mut transactions = vec![];

        if let Some(installed) = self.cache.index.get(module.qualifier()) {
            if prompt_yn("This module is already installed, reinstall?", false) {
                transactions.push(ModuleTransaction::Reinstall(installed.clone(), module.clone()));
            }
            else { return; }
        } else {
            transactions.push(ModuleTransaction::Install(module.clone(), Manual));
        }


        info!("Resolving dependencies...");
        let mut dependencies = DependencyResolver::new(&self.index, &self.cache.index);

        if let Err(e) = dependencies.resolve(module) {
            error!("{e}");
            return;
        }

        // Free if doing reinstall
        if let Some(installed) = self.cache.index.get(module.qualifier()) {
            dependencies.free(installed);
        }

        transactions.append(&mut dependencies.create_transactions());

        println!();

        transaction::transact(transactions, &mut self.cache, &Installer::new(CheckedShell::new(&self.config)))
    }

    /// Uninstalls a module from the system
    pub fn uninstall_module(&mut self, name: &str) {
        info!("Querying cache...");
        let modules = self.cache.index.query(name);

        let module = if let Some(m) = prompt_choice_module(
            &modules.iter().map(|i| &i.module).collect(),
            "Which module do you want to remove?")
            .and_then(|i| modules.get(i).copied()) { m } else {

            error!("Couldn't find installed module for '{name}', is it installed?");
            return;
        };

        let mut transactions = vec![ModuleTransaction::Remove(module.clone())];


        info!("Checking for dependents...");
        let dependents = self.cache.index.specific_dependents(module.qualifier());
        if !dependents.is_empty() {
            error!("Some other modules ({}) depend on this module",
                dependents.iter().map(|i| i.qualifier().unique()).collect::<Vec<String>>().join(", "));
            if !prompt_yn("Do you want to force removal?", false) {
                return;
            }
        }


        info!("Freeing dependencies...");
        let mut resolver = DependencyResolver::new(&self.index, &self.cache.index);
        resolver.free(module);
        transactions.append(&mut resolver.create_transactions());

        println!();

        transaction::transact(transactions, &mut self.cache, &Installer::new(CheckedShell::new(&self.config)))
    }

    /// Updates all modules
    pub fn update_everything(&mut self) {
        info!("Looking for updates...");
        let updatable: Vec<(&InstalledModule, Module)> = self.cache.index.modules.iter().filter_map(|installed| {

            if let Some(indexed) = self.index.query(&installed.module.qualifier.unique()).first() {
                if !installed.module.up_to_date(indexed) {
                    return Some((installed, indexed.deref().clone()))
                }
            }

            None
        }).collect();

        if updatable.is_empty() {
            info!("Everything is up-to-date, there is nothing to do");
            return;
        }


        info!("Calculating dependency changes...");
        let mut resolver = DependencyResolver::new(&self.index, &self.cache.index);

        for (i, m) in &updatable {
            if let Err(e) = resolver.resolve(m) {
                error!("Skipping update for {}: {e}", m.qualifier().unique());
                continue;
            }
            resolver.free(i);
        }


        info!("Creating transactions...");
        let mut transactions = vec![];

        for (i, m) in updatable {
            transactions.push(ModuleTransaction::Update(i.clone(), m.clone()));
        }
        transactions.append(&mut resolver.create_transactions());

        println!();

        transaction::transact(transactions, &mut self.cache, &Installer::new(CheckedShell::new(&self.config)));
    }

    /// Updates a single module
    pub fn update_module(&mut self, name: &str) {
        info!("Querying cache and sources...");

        let modules = self.cache.index.query(name);

        let module = if let Some(m) = prompt_choice_module(
            &modules.iter().map(|i| &i.module).collect(),
            "Which module do you want to update?")
            .and_then(|i| modules.get(i).copied()) { m } else {

            error!("No module under the name '{name}' is installed, try installing one first");
            return;
        };

        let indexed = if let Some(m) = self.index.get(&module.qualifier()) { m } else {
            info!("The module {} is orphaned, there is nothing to do", module.qualifier().unique());
            return;
        };

        if module.module.up_to_date(&indexed) {
            info!("The module {} is already up-to-date, there is nothing to do", module.qualifier().unique())
        }


        info!("Resolving dependency changes...");
        let mut resolver = DependencyResolver::new(&self.index, &self.cache.index);
        if let Err(e) = resolver.resolve(indexed) {
            error!("Cancelling update: {e}");
            return;
        }
        resolver.free(module);


        let mut transactions = vec![];
        transactions.append(&mut resolver.create_transactions());
        transactions.push(ModuleTransaction::Update(module.clone(), indexed.clone()));

        println!();

        transaction::transact(transactions, &mut self.cache, &Installer::new(CheckedShell::new(&self.config)));
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
            for module in &self.cache.index.modules {
                let naive: DateTime<Local> = module.installed.into();

                let orphaned = if self.index.query(&module.module.qualifier.unique()).is_empty() {
                    format!("-{}", "orphaned".red())
                } else {
                    String::default()
                };

                info!("{} ({}-{}{}) {} {}",
                    module.module.name.bold(),
                    module.module.qualifier.unique(),
                    module.module.version.dimmed(),
                    orphaned,
                    "at".italic(),
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