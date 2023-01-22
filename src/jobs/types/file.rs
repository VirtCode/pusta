use std::fmt::format;
use std::fs;
use std::path::PathBuf;
use anyhow::{Context, Error};
use log::info;
use serde::{Deserialize, Serialize};
use crate::jobs::{Installable, InstallReader, InstallWriter, JobCacheReader, JobCacheWriter, JobEnvironment};

#[derive(Serialize, Deserialize, Clone)]
pub struct FileJob {
    file: String,
    location: String,

    root: Option<bool>,
    link: Option<bool>
}

#[typetag::serde(name = "file")]
impl Installable for FileJob {

    fn install(&self, env: &JobEnvironment, writer: &mut InstallWriter) -> anyhow::Result<()> {
        let root = self.link.unwrap_or(false);

        // Get source file
        let mut file = env.module_path.clone();
        file.push(&self.file);
        if !file.exists() { return Err(Error::msg(format!("File ('{}') does not exist", file.to_string_lossy()))); }

        // Get target location
        let mut target = PathBuf::from(shellexpand::tilde(&self.location).as_ref());

        if target.exists() {
            // There is already a file at the target location
            writer.cache.cache_foreign(&target, "original");
            env.shell.remove(&target, root).context("Failed to remove original file to replace")?;

        } else if let Some(path) = target.parent() {
            if !path.exists() {
                // Parent dir does not yet exist
                env.shell.make_dir(path, root).context("Failed to make parent directories")?;

            } else if !path.is_dir() {
                // Parent dir is not a file
                return Err(Error::msg("Location parent directory is not a directory"))
            }
        }

        // Link or Copy file
        if self.link.unwrap_or(false) {
            env.shell.link(&file, &target, root).context("Failed to create symlink")?;
        } else {
            // TODO: process variables
            env.shell.copy(&file, &target, root).context("Failed to copy file")?;
        }

        // Mark used file as resource
        writer.resources.mark(self.file.clone());

        Ok(())
    }

    fn uninstall(&self, env: &JobEnvironment, reader: &InstallReader) -> anyhow::Result<()> {
        let mut target = PathBuf::from(shellexpand::tilde(&self.location).as_ref());

        if !target.exists() {
            // File was already removed
            return Err(Error::msg("Cannot revert file since it was removed"));
        }

        // Remove managed file
        env.shell.remove(&target, self.root.unwrap_or(false)).context("Failed to remove installed file")?;

        if let Some(original) = reader.cache.retrieve("original") {
            // Restore original file
            env.shell.copy(&original, &target, self.root.unwrap_or(false)).context("Failed to restore original file")?;
        }

        Ok(())
    }

    fn construct_title(&self) -> String {
        let action = self.link.unwrap_or(false);
        format!("{} the file '{}' to its target location",
            if action { "Linking" } else { "Copying" },
            &self.file
        )
    }
}