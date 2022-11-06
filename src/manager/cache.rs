use std::fs;
use std::fs::File;
use std::io::{Write};
use std::path::PathBuf;
use std::time::{Instant, SystemTime, };
use anyhow::Context;
use serde_with::{serde_as, TimestampMilliSeconds};
use log::{error, info, warn};
use serde::{Deserialize, Serialize};
use serde_with::formats::Flexible;
use crate::module::Module;
use crate::module::repository::Repository;

pub const CACHE_FILE: &str = "~/.config/pusta/cache/installed.json";

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
    qualifier: String,
    checksum: String,
    #[serde_as(as = "TimestampMilliSeconds<String, Flexible>")]
    install_time: SystemTime,
    #[serde_as(as = "TimestampMilliSeconds<String, Flexible>")]
    update_time: SystemTime
}

impl Cache {
    pub fn read() -> Self {
        info!("Reading cache file with installed modules/repositories");
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

    pub fn installed_module(&mut self, m: &Module) -> anyhow::Result<()> {
        self.modules.push(CacheModule {
            qualifier: m.unique_qualifier(),
            checksum: m.current_checksum(),
            install_time: SystemTime::now(),
            update_time: SystemTime::now()
        });

        self.write()
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
        let repo = self.repos.iter().find(|r| &r.alias == alias);

        if let Some(repo) = repo {
            let alias = repo.alias.clone();

            self.repos.retain(|r| r.alias != alias);
            
            self.write()?;

            Ok(true)
        } else { Ok(false) }
    }
}