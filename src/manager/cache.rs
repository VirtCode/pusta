use std::fs;
use std::fs::File;
use std::io::{Write};
use std::path::{Path, PathBuf};
use std::time::{Instant, SystemTime, };
use anyhow::Context;
use serde_with::{serde_as, TimestampMilliSeconds};
use log::{debug, error, info, warn};
use serde::{Deserialize, Serialize};
use serde_with::formats::Flexible;
use crate::module::Module;
use crate::module::repository::Repository;

pub const CACHE_FILE: &str = "~/.config/pusta/cache/installed.json";
pub const CACHE_LOCATION: &str = "~/.config/pusta/cache/";

#[derive(Deserialize, Serialize)]
pub struct Cache {
    pub repos: Vec<CacheRepository>,
    pub modules: Vec<CacheModule>
}

#[serde_as]
#[derive(Deserialize, Serialize)]
pub struct CacheRepository {
    pub path: PathBuf,
    pub alias: String,
    #[serde_as(as = "TimestampMilliSeconds<String, Flexible>")]
    install_time: SystemTime
}

#[serde_as]
#[derive(Deserialize, Serialize)]
pub struct CacheModule {
    pub qualifier: String,
    pub checksum: String,
    #[serde_as(as = "TimestampMilliSeconds<String, Flexible>")]
    install_time: SystemTime,
    #[serde_as(as = "TimestampMilliSeconds<String, Flexible>")]
    update_time: SystemTime
}

impl CacheModule {
    pub fn qualifies(&self, qualifier: &str) -> bool {
        if self.qualifier == qualifier { true }
        else {
            let result: Vec<&str> = self.qualifier.split('/').collect();

            result.len() == 2 && result.get(1).unwrap() == &qualifier
        }
    }
}

impl Cache {
    pub fn read() -> Self {
        info!("Reading installed modules and repositories...");
        if let Ok(c) = Self::try_read() {
            c
        } else {
            warn!("No cache file found, creating new one");
            let cache = Cache {
                repos: vec![],
                modules: vec![]
            };

            if let Err(e) = cache.write() {
                error!("Failed to firstly create cache file. {}", e.to_string());
            }

            cache
        }
    }

    fn try_read() -> anyhow::Result<Self> {
        let path = PathBuf::from(shellexpand::tilde(CACHE_FILE).to_string());

        let f = File::open(&path).with_context(|| format!("Couldn't open cache installed file at '{}'", path.to_string_lossy()))?;
        serde_json::from_reader(f).with_context(|| format!("Couldn't read cache installed file from '{}'", path.to_string_lossy()))
    }

    fn write(&self) -> anyhow::Result<()>{

        let path = PathBuf::from(shellexpand::tilde(CACHE_FILE).to_string());

        fs::create_dir_all(path.parent().context("Failed to get parent dir for installed cache file")?).context("Failed to create dirs for cache files")?;
        let f = File::create(&path).with_context(|| format!("Couldn't create cache installed file '{}'", path.to_string_lossy()))?;

        serde_json::to_writer(&f, self).with_context(|| format!("Couldn't write cache installed file to '{}'", path.to_string_lossy()))
    }

    pub fn add_module(&mut self, m: &Module, actions: Vec<InstalledAction>) -> anyhow::Result<()> {
        let checksum = m.current_checksum();

        debug!("Saving installed actions");
        let mut path = PathBuf::from(shellexpand::tilde(CACHE_LOCATION).to_string());
        path.push(checksum.clone());
        fs::create_dir_all(&path).with_context(|| format!("Couldn't create module cache dir '{}'", path.to_string_lossy()))?;

        let mut actions_path = path.clone();
        actions_path.push("actions.json");
        let actions_file = File::create(&actions_path).with_context(|| format!("Couldn't create actions file at '{}'", actions_path.to_string_lossy()))?;

        serde_json::to_writer(&actions_file, &actions).with_context(|| format!("Couldn't write actions to '{}'", actions_path.to_string_lossy()))?;

        debug!("Saving necessary files for uninstallation");
        for action in actions {
            if let InstalledAction::Script { uninstall, .. } = action {
                let mut target = path.clone();
                target.push(&uninstall);

                let mut source = m.path.clone();
                source.push(uninstall);

                fs::copy(&source, &target).with_context(|| format!("Failed to copy file to cache ({} -> {})", source.to_string_lossy(), target.to_string_lossy()))?;
            }
        }

        debug!("Adding module entry to cache");
        self.modules.push(CacheModule {
            qualifier: m.unique_qualifier(),
            checksum,
            install_time: SystemTime::now(),
            update_time: SystemTime::now()
        });

        self.write()?;

        Ok(())
    }

    pub fn get_module(&self, qualifier: &str) -> Option<&CacheModule> {
        self.modules.iter().find(|m| m.qualifies(qualifier))
    }

    pub fn data_module(&self, qualifier: &str) -> anyhow::Result<Option<(&CacheModule, Vec<InstalledAction>, PathBuf)>> {
        let module = self.get_module(qualifier);
        if let Some(module) = module {

            let mut path = PathBuf::from(shellexpand::tilde(CACHE_LOCATION).to_string());
            path.push(&module.checksum);

            let mut actions_path = path.clone();
            actions_path.push("actions.json");

            debug!("Reading actions file for module {}", &module.qualifier);
            let file = File::open(actions_path).context("Failed to open actions file for module")?;
            let actions: Vec<InstalledAction> = serde_json::from_reader(file).context("Failed to read from actions file of module")?;

            Ok(Some((module, actions, path)))
        } else { Ok(None) }
    }

    pub fn remove_module(&mut self, qualifier: &str) -> anyhow::Result<bool> {
        let module = self.get_module(qualifier);

        if let Some(module) = module {

            debug!("Deleting version cache of module");
            let mut path = PathBuf::from(shellexpand::tilde(CACHE_LOCATION).to_string());
            path.push(&module.checksum.clone());
            fs::remove_dir_all(path).context("Failed to remove cache directory of module")?;

            // Create new checksum instance, required to not use immutable borrow anymore
            let checksum = module.checksum.clone();

            debug!("Removing module from installed cache");
            self.modules.retain(|m| m.checksum != checksum);
            self.write()?;

            Ok(true)
        } else { Ok(false) }
    }

    pub fn add_repo(&mut self, r: &Repository) -> anyhow::Result<()> {
        self.repos.push(CacheRepository {
            path: r.location.clone(),
            alias: r.name.clone(),
            install_time: SystemTime::now()
        });

        self.write()
    }

    pub fn remove_repo(&mut self, alias: &str) -> anyhow::Result<bool> {
        let repo = self.repos.iter().find(|r| r.alias == alias);

        if let Some(repo) = repo {
            let alias = repo.alias.clone();

            self.repos.retain(|r| r.alias != alias);
            
            self.write()?;

            Ok(true)
        } else { Ok(false) }
    }
}