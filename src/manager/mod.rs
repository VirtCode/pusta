use std::path::PathBuf;
use log::{debug, error, info};
use crate::config::Config;
use crate::manager::cache::Cache;
use crate::manager::registry::Registry;
use crate::module::install::shell::Shell;

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
            info!("Installing module {}...", module.unique_qualifier());

            match module.install(shell) {
                Ok(actions) => {
                    self.cache.installed_module(module, actions)?;

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


}

