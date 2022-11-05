use std::fs;
use std::fs::File;
use std::io::{Write};
use std::path::PathBuf;
use std::time::{Instant, SystemTime};
use anyhow::Context;
use log::error;
use serde::{Deserialize, Serialize};
use crate::module::Module;

pub const CACHE_FILE: &str = "~/.config/pusta/cache/installed.json";

#[derive(Deserialize, Serialize)]
pub struct Cache {
    repos: Vec<CacheRepository>,
    module: Vec<CacheModule>
}

#[derive(Deserialize, Serialize)]
pub struct CacheRepository {
    path: PathBuf,
    alias: String,
    install_time: SystemTime
}

#[derive(Deserialize, Serialize)]
pub struct CacheModule {
    qualifier: String,
    checksum: String,
    install_time: SystemTime,
    update_time: SystemTime
}

impl Cache {
    pub fn read() -> Self {
        if let Ok(c) = Self::try_read() {
            c
        } else {
            let cache = Cache {
                repos: vec![],
                module: vec![]
            };

            if let Err(e) = cache.write() {
                println!("Failed to firstly create cache file. {}", e.to_string());
            }

            cache
        }
    }

    fn try_read() -> anyhow::Result<Self> {
        let path = PathBuf::from(shellexpand::tilde(CACHE_FILE).to_string());

        let f = File::open(&path).with_context(|| format!("Couldn't open cache installed file at '{}'", path.to_string_lossy()))?;
        serde_json::from_reader(f).with_context(|| format!("Couldn't read cache installed file from '{}'", path.to_string_lossy()))
    }

    // FIXME: Make private, no interaction from outside
    pub fn write(&self) -> anyhow::Result<()>{

        let path = PathBuf::from(shellexpand::tilde(CACHE_FILE).to_string());

        fs::create_dir_all(path.parent().context("Failed to get parent dir for installed cache file")?).context("Failed to create dirs for cache files")?;
        let f = File::create(&path).with_context(|| format!("Couldn't create cache installed file '{}'", path.to_string_lossy()))?;

        serde_json::to_writer(&f, self).with_context(|| format!("Couldn't write cache installed file to '{}'", path.to_string_lossy()))
    }

    pub fn installed_module(&mut self, m: &Module) {
        self.module.push(CacheModule {
            qualifier: m.unique_qualifier(),
            checksum: m.current_checksum(),
            install_time: SystemTime::now(),
            update_time: SystemTime::now()
        })
    }
}