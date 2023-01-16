mod index;
pub mod cache;

use std::ops::Deref;
use std::path::{Path, PathBuf};
use anyhow::{Error, format_err};
use colored::Colorize;
use log::{error, info, warn};
use crate::config::Config;
use crate::module::install::Installer;
use crate::module::install::neoshell::Shell;
use crate::module::repository::Repository;
use crate::output;
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
            else { output::prompt_choice("Which module do you mean?", &modules.iter().map(|m| format!("{} ({})", m.qualifier.unique(), &m.name)).collect(), None) };

        let module = modules.get(index).expect("index math went wrong");

        // Make sure not already installed
        if self.cache.has_module(&module.qualifier.unique()) {
            error!("This module is already installed on your system");
            return;
        }

        // Collect modules
        let modules = vec![module.deref().clone()]; // Copy now, since it is used for sure here

        info!("Resolving dependencies...");
        warn!("not yet implemented");

        // Prompt user for confirmation
        println!();
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

        // Do installation
        let mut installed = vec![];
        let installer = Installer::new(Shell::new(&self.config));

        for module in modules {
            let unique = module.qualifier.unique();

            if let Some(m) = installer.install(module, &self.cache) {
                installed.push(m);
            } else {
                warn!("Failed to install module '{unique}'");
                if !output::prompt_yn("Do you want to continue with the installation of the rest?", true) {
                    break;
                }
            }
        }

        for installed in installed {
            self.cache.install_module(installed).unwrap_or_else(|e| {
                error!("Error whilst persisting install: {}", e.to_string());
            })
        }

        info!("Installation finished")
    }
}