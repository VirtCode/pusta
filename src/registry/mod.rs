mod index;
pub mod cache;
mod transaction;

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
use crate::output::{logger, prompt_yn};
use crate::output::logger::{disable_indent, enable_indent};
use crate::registry::cache::Cache;
use crate::registry::index::Index;
use crate::registry::transaction::ModuleTransaction;

pub struct Registry {
    index: Index,
    cache: Cache,
    config: Config
}

impl Registry {
    pub fn new(config: &Config) -> Self {
        Registry {
            index: Index::new(),
            cache: Cache::new(&PathBuf::from(shellexpand::tilde(crate::CACHE).to_string())),
            config: (*config).clone()
        }
    }

    pub fn load(&mut self) -> anyhow::Result<()> {
        self.cache.load();

        self.index.load_repositories(&self.cache.repositories)
    }

    // Adds a repository
    pub fn add(&mut self, repository: &Path, alias: Option<&str>) {
        info!("Adding repository at '{}' to sources{}",
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
            error!("{}", e.to_string());
            return;
        }

        info!("Loading modules from the repository");
        enable_indent();
        if let Err(e) = self.index.load_repository(self.cache.repository(&name).expect("just added repo went missing")) {
            warn!("Failed to load modules from this repository: {}", e.to_string());
        }
        disable_indent();

        info!("Successfully added repository")
    }

    // Removes a repository
    pub fn unadd(&mut self, name: &str) {
        info!("Removing source repository under alias '{name}'");

        if let Some(r) = self.cache.remove_repository(name) {

            self.index.unload_repository(&r);
            info!("Successfully removed repository")
        } else {
            error!("Failed to remove, no repository with this alias found")
        }
    }

    pub fn choose_one(modules: &Vec<&Module>, prompt: &str) -> Option<usize> {

        match modules.len() {
            0 => None,
            1 => Some(0usize),
            _ => {
                Some(output::prompt_choice(
                    prompt,
                    &modules.iter().map(|m| format!("{} ({})", m.qualifier.unique(), &m.name)).collect(),
                    None))
            }
        }
    }

    pub fn resolve_dependencies(&self, module: &Module, scheduled_modules: &mut Vec<ModuleQualifier>, new_transactions: &mut Vec<ModuleTransaction>) -> bool {
        scheduled_modules.push(module.qualifier.clone());

        for dep in &module.dependencies {
            if self.cache.has_provider(dep) || scheduled_modules.iter().any(|m| m.does_provide(dep)) { continue }

            let providers = self.index.providers(dep);
            if let Some(m) = Registry::choose_one(
                &providers,
                &format!("Multiple modules provide dependency '{dep}' for {}, choose:", module.qualifier.unique())).and_then(|i| providers.get(i).copied()) {

                new_transactions.push(ModuleTransaction::Install(m.clone(), Dependency));
                self.resolve_dependencies(m, scheduled_modules, new_transactions);

            } else {
                error!("Failed to find module for dependency '{dep}' required by {}", module.qualifier.unique());
                if !prompt_yn("Continue without this dependency?", false) { return false }
            }
        }

        true
    }

    // Removes all dependencies which are no longer required, important: removed takes modules which are no longer counted as dependencies because they are being removed
    pub fn free_dependencies(&self, freeable: &Vec<String>, removed: &mut Vec<ModuleQualifier>, new_transactions: &mut Vec<ModuleTransaction>) {
        for dep in freeable.iter()
            .flat_map(|s| self.cache.find_providers(s).into_iter())
            .filter(|i| matches!(i.reason, Dependency)) {
            if removed.iter().any(|r| *r == dep.module.qualifier) { continue }

            if self.cache.depended_upon(dep, removed).is_none() &&
                prompt_yn(&format!("The module {} is no longer being depended upon, remove?", dep.module.qualifier.unique()), false){

                removed.push(dep.module.qualifier.clone());
                new_transactions.push(ModuleTransaction::Remove(dep.clone()));
                self.free_dependencies(&dep.module.dependencies, removed, new_transactions);
            }
        }
    }

    pub fn install(&mut self, name: &str) {
        info!("Querying sources...");
        let modules = self.index.query(name);

        let module = if let Some(m) =
            Registry::choose_one(&modules, "Which module do you mean?")
                .and_then(|i| modules.get(i).copied()) { m } else {

            error!("Couldn't find a module under this name, are relevant sources added?");
            return;
        };

        let mut transactions = vec![];

        if let Some(installed) = self.cache.find_module(&module.qualifier.unique()) {
            if prompt_yn("This module is already installed, reinstall?", false) {
                transactions.push(ModuleTransaction::Reinstall(installed.clone(), module.clone()));
            }
            else { return; }
        } else {
            transactions.push(ModuleTransaction::Install(module.clone(), Manual));
        }


        info!("Resolving dependencies...");
        self.resolve_dependencies(module, &mut vec![], &mut transactions);

        println!();

        transaction::transact(transactions, &mut self.cache, &Installer::new(CheckedShell::new(&self.config)))
    }

    pub fn remove(&mut self, name: &str) {
        info!("Querying cache...");
        let modules = self.cache.query_module(name);

        let module = if let Some(m) = Registry::choose_one(
            &modules.iter().map(|i| &i.module).collect(),
            "Which module do you want to remove?")
            .and_then(|i| modules.get(i).copied()) { m } else {

            error!("Couldn't find installed module for '{name}', is it installed?");
            return;
        };

        let mut transactions = vec![ModuleTransaction::Remove(module.clone())];


        info!("Checking for dependents...");
        if let Some(depender) = self.cache.depended_upon(module, &vec![]) {
            error!("The module {} and possibly others still depend on this module", depender.module.qualifier.unique());
            if !prompt_yn("Do you want to force removal?", false) {
                return;
            }
        }


        info!("Freeing dependencies...");
        self.free_dependencies(&module.module.dependencies, &mut vec![module.module.qualifier.clone()], &mut transactions);

        println!();

        transaction::transact(transactions, &mut self.cache, &Installer::new(CheckedShell::new(&self.config)))
    }

    pub fn update_all(&mut self) {
        info!("Looking for updates...");
        let updatable: Vec<(&InstalledModule, Module)> = self.cache.modules.iter().filter_map(|installed| {

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

        let mut transactions = vec![];


        info!("Applying dependency changes and creating transactions...");
        let mut freed: Vec<ModuleQualifier> = updatable.iter().map(|(_i, m)| m.qualifier.clone()).collect();
        for (i, m) in &updatable {
            // Free before everything else so a dependency is not freed directly after install
            let removed: Vec<String> = i.module.dependencies.iter().filter(|s| !m.dependencies.contains(*s)).cloned().collect();
            self.free_dependencies(&removed, &mut freed, &mut transactions);
        }

        let mut scheduled: Vec<ModuleQualifier> = updatable.iter().map(|(_i, m)| m.qualifier.clone()).collect();
        for (i, m) in updatable {

            if !self.resolve_dependencies(&m, &mut scheduled, &mut transactions) {
                continue;
            }

            transactions.push(ModuleTransaction::Update(i.clone(), m));
        }

        println!();

        transaction::transact(transactions, &mut self.cache, &Installer::new(CheckedShell::new(&self.config)));
    }

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

        if self.cache.modules.is_empty() {
            info!("{}", "No modules are currently installed".italic().dimmed())
        } else {
            for module in &self.cache.modules {
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

    pub fn query(&self, query: &str) {
        let modules = self.index.query(query);

        if modules.is_empty() {
            info!("{}", "No modules qualify for this query".dimmed().italic())
        } else {
            for module in modules {
                let installed = if !self.cache.query_module(&module.qualifier.unique()).is_empty() {
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