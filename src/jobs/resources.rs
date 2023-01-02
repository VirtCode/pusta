use std::fs::File;
use std::path::Path;
use anyhow::Context;
use chksum::Chksum;
use chksum::hash::HashAlgorithm;
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

        self.resources.iter().map(|s| {
            // Get absolute path
            let mut path = folder.to_owned();
            path.push(s);
            path.canonicalize().context("Failed to canonicalize resource path")?;

            // Calculate checksum
            let checksum = format!("{:x}", File::open(path).context("Failed to open file to get checksum")?.chksum(HashAlgorithm::SHA1).context("Failed to calculate checksum")?);;

            Ok(ResourceFile {
                file: s.clone(),
                checksum,
            })
        }).filter_map(|r: anyhow::Result<ResourceFile>| {
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

pub struct ResourceFile {
    file: String,
    checksum: String
}