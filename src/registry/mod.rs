mod index;
pub mod cache;

use std::ops::Deref;
use std::os::unix::raw::time_t;
use std::path::{Path, PathBuf};
use anyhow::{Error, format_err};
use chrono::{DateTime, Local, NaiveDateTime};
use colored::Colorize;
use log::{debug, error, info, warn};
use crate::config::Config;
use crate::module::install::checked::CheckedShell;
use crate::module::install::{InstalledModule, Installer};
use crate::module::install::shell::Shell;
use crate::module::Module;
use crate::module::repository::Repository;
use crate::output;
use crate::output::logger;
use crate::output::logger::{disable_indent, enable_indent};
use crate::registry::cache::Cache;
use crate::registry::index::Index;

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

    pub fn unadd(&mut self, name: &str) {
        info!("Removing source repository under alias '{name}'");

        if let Some(r) = self.cache.remove_repository(name) {

            self.index.unload_repository(&r);
            info!("Successfully removed repository")
        } else {
            error!("Failed to remove, no repository with this alias found")
        }
    }

    pub fn install(&mut self, name: &str) {
        // Find packages
        let modules = self.index.query(name);
        if modules.is_empty() {
            error!("Couldn't find module for '{name}', make sure it is spelled correctly and relevant sources are added");
            return;
        }

        let index =
            if modules.len() == 1 { 0 }
            else { output::prompt_choice("Which module do you want to install?", &modules.iter().map(|m| format!("{} ({})", m.qualifier.unique(), &m.name)).collect(), None) };

        if modules.len() > 1 { println!(); }
        let module = modules.get(index).expect("index math went wrong");

        // Make sure not already installed
        if self.cache.has_module(&module.qualifier.unique()) {
            error!("This module is already installed on your system");
            return;
        }

        // Collect modules
        let modules = vec![module.deref().clone()]; // Copy now, since it is used for sure here

        debug!("Resolving dependencies...");
        // TODO: Check for dependencies

        // Prompt user for confirmation
        info!("Modules scheduled for install:");
        for module in &modules {
            println!("   {} ({}-{})",
                     module.name.bold(),
                     module.qualifier.unique(),
                     module.version.dimmed());
        }

        if !output::prompt_yn("Do you want to install these modules now?", true) {
            error!("Installation cancelled by user");
            return;
        }

        println!();

        // Do installation
        let mut installed = vec![];
        let installer = Installer::new(CheckedShell::new(&self.config));

        let amount = modules.len();
        for (i, module) in modules.into_iter().enumerate() {
            let unique = module.qualifier.unique();

            output::start_section(&format!("Installing module '{unique}'"));

            if let Some(m) = installer.install(module, &self.cache) {
                output::end_section(true, &format!("Successfully installed module '{unique}'"));
                installed.push(m);
            } else {
                output::end_section(false, &format!("Failed to install module '{unique}'"));
                if i == amount - 1 || !output::prompt_yn("Do you want to continue with the installation of the rest?", true) {
                    break;
                }
            }
        }

        for installed in installed {
            self.cache.install_module(installed).unwrap_or_else(|e| {
                error!("Error whilst persisting install: {}", e.to_string());
            })
        }

        info!("Finished installing all modules")
    }

    pub fn remove(&mut self, name: &str) {
        let modules = self.cache.query_module(name);
        if modules.is_empty() {
            error!("Couldn't find installed module for '{name}', make sure one is installed");
            return;
        }

        let index =
            if modules.len() == 1 { 0 }
            else { output::prompt_choice("Which module do you want to remove?", &modules.iter()
                .map(|m| format!("{} ({})", m.module.qualifier.unique(), &m.module.name))
                .collect(), None) };

        if modules.len() > 1 { println!(); }
        let module = *modules.get(index).expect("index math went wrong");

        debug!("Checking for dependents");
        // TODO: Check for dependents

        // Prompt user for confirmation
        info!("Module scheduled for uninstall: {} ({}-{})",
            module.module.name.bold(),
            module.module.qualifier.unique(),
            module.module.version.dimmed());

        if !output::prompt_yn("Do you want to remove this module now?", true) {
            error!("Removal canceled by user");
            return;
        }

        println!();

        let installer = Installer::new(CheckedShell::new(&self.config));
        output::start_section(&format!("Removing module '{}' ...", module.module.qualifier.unique()));
        installer.uninstall(module, &self.cache);
        output::end_section(true, "Finished removal of module");

        self.cache.delete_module_cache(&module.module).unwrap_or_else(|e| {
            debug!("Failed to delete module cache ({}), filesystem may stay polluted", e.to_string());
        });
        self.cache.remove_module(&module.module.qualifier.unique());

        info!("Finished removing module");
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


        // Prompt user for confirmation
        println!();
        info!("Modules scheduled for udpate:");
        for (installed, new) in &updatable {
            println!("   {} ({}-{} -> {}-{})",
                     installed.module.name.bold(),
                     installed.module.qualifier.unique(),
                     installed.module.version.dimmed(),
                     new.qualifier.unique(),
                     new.version.dimmed());
        }

        if !output::prompt_yn("Do you want to update these modules now?", true) {
            error!("Update cancelled by user");
            return;
        }

        println!();

        let mut results = vec![];

        let installer = Installer::new(CheckedShell::new(&self.config));
        for (installed, new) in updatable {
            output::start_section(&format!("Updating module '{}'", installed.module.qualifier.unique()));

            let result = installer.update(installed, new, &self.cache);

            if let Some(option) = result {
                if option.is_some() {
                    output::end_section(true, &format!("Successfully updated module '{}'", installed.module.qualifier.unique()));
                } else {
                    output::end_section(false, &format!("Failure occurred during updating the module '{}', it is no longer installed", installed.module.qualifier.unique()));
                }

                results.push((installed.module.qualifier.unique(), option));
            } else {
                output::end_section(false, &format!("Couldn't update module '{}'", installed.module.qualifier.unique()));
            }
        }

        // Persist changes in cache
        for (old, new) in results {
            self.cache.remove_module(&old);

            if let Some(new) = new {
                self.cache.install_module(new).unwrap_or_else(|e| {
                    error!("Error whilst persisting install: {}", e.to_string());
                })
            }
        }
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