use std::fs::File;
use std::path::{Path, PathBuf};
use anyhow::Context;
use chksum::chksum;
use chksum::hash::SHA1;
use serde::{Deserialize, Serialize};
use log::warn;

/// This struct is used for a job to mark out which resources it depends on
/// If such a resource changes, that specific job is rerun
pub struct JobResources {
    resources: Vec<String>
}

impl JobResources {
    pub fn new() -> Self {
        Self {
            resources: vec![]
        }
    }

    pub fn mark(&mut self, file: String) {
        self.resources.push(file);
    }

    pub fn process(&self, folder: &Path) -> Vec<ResourceFile>{

        self.resources.iter()
            .map(|s| ResourceFile::process(s.clone(), folder))
            .filter_map(|r: anyhow::Result<ResourceFile>| {
            // Only retain successes
            match r {
                Ok(f) => Some(f),
                Err(e) => {
                   warn!("Failed to process resource: {}", e);
                   None
                }
            }
        }).collect()

    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ResourceFile {
    file: String,
    checksum: String
}

impl ResourceFile {

    fn process(file: String, folder: &Path) -> anyhow::Result<Self> {
        // Get absolute path
        let mut path = folder.to_owned();
        path.push(&file);
        path.canonicalize().context("Failed to canonicalize resource path")?;

        // Calculate checksum
        let handle = File::open(path).context("Failed to open file to get checksum")?;
        let checksum = chksum::<SHA1, _>(handle).context("Failed to calculate checksum")?.to_hex_lowercase();

        Ok(ResourceFile {
            file,
            checksum,
        })
    }

    pub fn up_to_date(&self, folder: &Path) -> anyhow::Result<bool> {
        // Get absolute path
        let mut path = folder.to_owned();
        path.push(&self.file);
        path.canonicalize().context("Failed to canonicalize resource path")?;

        // Calculate checksum
        let handle = File::open(path).context("Failed to open file to get checksum")?;
        let checksum = chksum::<SHA1, _>(handle).context("Failed to calculate checksum")?.to_hex_lowercase();

        return Ok(self.checksum == checksum)
    }
}

pub struct ResourceItem {
    /// path the file is located at
    path: PathBuf,
    /// whether the checksum was drawn imminently
    imminent: bool,
    /// checksum of installed version
    checksum: String
}

impl ResourceItem {
    pub fn remove_me() -> Self {
        Self {
            path: Default::default(),
            imminent: false,
            checksum: "".to_string(),
        } 
    }
    
}