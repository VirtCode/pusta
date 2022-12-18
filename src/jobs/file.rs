use std::fs;
use std::path::PathBuf;
use anyhow::{Context, Error};
use serde::{Deserialize, Serialize};
use crate::jobs::{Installable, JobCacheReader, JobCacheWriter, JobEnvironment};

#[derive(Serialize, Deserialize)]
pub struct FileJob {
    file: String,
    location: String,

    root: Option<bool>,
    link: Option<bool>
}

#[typetag::serde(name = "file")]
impl Installable for FileJob {

    fn install(&self, env: &JobEnvironment, cache: &JobCacheWriter) -> anyhow::Result<()> {
        let link = self.link.unwrap_or(false);
        let root = self.link.unwrap_or(false);

        // Get source file
        let mut file = env.module_path.clone();
        file.push(&self.file);
        if !file.exists() { return Err(Error::msg(format!("File ('{}') does not exist", file.to_string_lossy()))); }

        // Get target location
        let mut target = PathBuf::from(shellexpand::tilde(&self.location).as_ref());

        if target.exists() {
            // There is already a file at the target location

        } else if let Some(path) = target.parent() {
            if !path.exists() {
                // Parent dir does not yet exist
                env.shell.make_dir(path, root).context("Failed to make parent directories")?;

            } else if !path.is_dir() {
                // Parent dir is not a file
                return Err(Error::msg("Location parent directory is not a directory"))
            }
        }



        Ok(())


    }

    fn uninstall(&self, env: &JobEnvironment, cache: &JobCacheReader) -> anyhow::Result<()> {
        todo!()
    }

    fn construct_title(&self) -> String {
        todo!()
    }
}