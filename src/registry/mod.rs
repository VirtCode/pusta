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
use crate::output::table::{table, Column};
use crate::registry::cache::Cache;
use crate::registry::index::{Index, Indexable};
use crate::variables::{construct_injected, generate_magic, load_system, Variable};

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

    pub fn newest_injected_variables(&self) -> Variable {
        let installed_newest = self.cache.index.modules.iter().map(|installed| {
            self.index.get(installed.qualifier()).unwrap_or(&installed.module) // orphaned are the newest already
        }).collect::<Vec<_>>();

        construct_injected(installed_newest)
    }

    /// Updates all modules
    pub fn update_everything(&mut self) {
        section("Looking for updates...");
        let magic = generate_magic();
        let system = load_system(&self.config).unwrap_or(Variable::base());
        let injected =  self.newest_injected_variables();

        let updatable: Vec<ModuleQualifier> = self.cache.index.modules.iter().filter_map(|installed| {

            if let Some(indexed) = self.index.get(installed.qualifier()) {
                if !installed.up_to_date(indexed, &magic, &system, &injected, &self.cache) {
                    info!("Found outdated module {}", installed.qualifier().unique());
                    return Some(installed.qualifier().clone())
                }
            }

            None
        }).collect();

        if updatable.is_empty() {
            section("Everything is already up to date!");
            return;
        }

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
        info!("{}", "Added source repositories:".underline().bold());

        if self.cache.repositories.is_empty() {
            info!("{}", "No sources are currently added".italic().dimmed())
        } else {
            let columns = [
                Column::new("Alias").force(),
                Column::new("Location").ellipse(),
            ];

            let rows = self.cache.repositories.iter().map(|repo| {
                [
                    repo.name.bold(),
                    repo.location.to_string_lossy().normal()
                ]
            }).collect();

            table(columns, rows, "  ");
        }
        println!();

        info!("{}", "Installed modules:".underline().bold());

        if self.cache.index.modules.is_empty() {
            info!("{}", "No modules are currently installed".italic().dimmed())
        } else {
            let magic = generate_magic();
            let system = load_system(&self.config).unwrap_or(Variable::base());
            let injected = self.newest_injected_variables();

            let mut sorted = self.cache.index.modules.iter().collect::<Vec<_>>();
            sorted.sort_by(|a, b| {
                a.module.qualifier.unique().cmp(&b.module.qualifier.unique())
            });

            let columns = [
                Column::new("Name").ellipse(),
                Column::new("Qualifier").force(),
                Column::new("Version"),
                Column::new("Status").force(),
                Column::new("Added").force()
            ];

            let rows = sorted.iter().map(|module| {
                let info = if let Some(indexed) = self.index.get(&module.module.qualifier) {
                    if !module.up_to_date(indexed, &magic, &injected, &system, &self.cache) {
                        "outdated".yellow()
                    } else {
                        "up-to-date".green()
                    }
                } else {
                    "orphaned".red()
                };

                let naive: DateTime<Local> = module.built.time.into();

                [
                    module.module.name.bold(),
                    module.module.qualifier.unique().normal(),
                    module.module.version.dimmed(),
                    info,
                    naive.format("%x").to_string().italic()
                ]
            }).collect();

            table(columns, rows, "  ");
        }
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
