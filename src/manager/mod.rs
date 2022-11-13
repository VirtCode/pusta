use std::fmt::format;
use std::path::PathBuf;
use log::{debug, error, info, warn};
use crate::config::Config;
use crate::manager::cache::Cache;
use crate::manager::registry::Registry;
use crate::module::install::shell::Shell;
use crate::module::uninstall;
use crate::output::prompt_yn;

mod registry;
mod cache;

pub struct Manager {
    registry: Registry,
    cache: Cache
}

impl Manager {
    pub fn load(config: &Config) -> Self {
        let mut manager = Manager {
            cache: Cache::read(),
            registry: Registry::new(config)
        };

        debug!("Adding repositories from cache");
        manager.load_repositories();

        manager
    }

    fn load_repositories(&mut self) {
        for repo in &self.cache.repos {

            debug!("Loading the repo '{}' from {}", &repo.alias, repo.path.to_string_lossy());
            self.registry.add(&repo.path, Some(&repo.alias)).map(|_| {}).unwrap_or_else(|e| {
                error!("Failed to add previously installed repository '{}' from '{}'\n    Reason: {}", &repo.alias, repo.path.to_string_lossy(), e.to_string())
            });

        }
    }

    pub fn add_repository(&mut self, path: &PathBuf, alias: Option<&String>) -> anyhow::Result<()> {
        debug!("Loading and reading repository from '{}'", path.to_string_lossy());
        let repo = self.registry.add(path, alias)?;
        debug!("Successfully read and loaded repository '{}'", &repo.name);

        self.cache.add_repo(repo)?;

        Ok(())
    }

    pub fn remove_repository(&mut self, alias: &str) -> anyhow::Result<bool>{
        let result = self.cache.remove_repo(alias)?;
        
        if result { self.registry.remove(alias); }
        
        Ok(result)
    }
    
    pub fn install_module(&mut self, qualifier: &str, shell: &Shell) -> anyhow::Result<bool> {
        info!("Resolving '{}' in added repositories", qualifier);
        let module = self.registry.get(qualifier);
        
        if let Some(module) = module {
            if let Some(installed) = self.cache.get_module(&module.unique_qualifier()) {
                error!("{} module is already installed, try removing it first", if installed.checksum == module.current_checksum() { "This" } else { "Another version of this" });
                return Ok(false)
            }

            if !prompt_yn(&format!("Do you want to install the module '{}'?", &module.unique_qualifier()), true) {
                return Ok(false);
            }

            info!("Installing module {}...", module.unique_qualifier());

            match module.install(shell) {
                Ok(actions) => {
                    self.cache.add_module(module, actions)?;

                    info!("");
                    info!("Successfully installed module {}.", module.unique_qualifier());

                    Ok(true)
                }
                Err(e) => {
                    info!("");
                    error!("Failed to install module {}: {}", module.unique_qualifier(), e);

                    Ok(false)
                }
            }
        } else {
            error!("Could not find a module qualifying for '{}'", qualifier);
            Ok(false)
        }
    }

    pub fn uninstall_module(&mut self, qualifier: &str, shell: &Shell) -> anyhow::Result<bool>{
        info!("Resolving '{}' in installed modules", qualifier);
        if let Some((module, actions, path)) = self.cache.data_module(qualifier)? {

            if !prompt_yn(&format!("Do you want to uninstall the module '{}'?", &module.qualifier), true) {
                return Ok(false);
            }
            info!("Removing module {}...", module.qualifier);

            match uninstall(actions, &path, shell) {
                Ok(_) => {
                    info!("");
                    info!("Successfully removed module {} from system.", module.qualifier);
                }
                Err(e) => {
                    info!("");
                    warn!("Could not completely remove module {} from system.", module.qualifier);
                }
            };

            info!("Clearing module cache");
            self.cache.remove_module(&module.qualifier.clone())?;

            Ok(true)
        } else {
            error!("No module under this qualifier is currently installed");
            Ok(false)
        }
    }
}

