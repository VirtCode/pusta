use std::fmt::format;
use std::fs;
use std::path::{Path, PathBuf};
use anyhow::{Context, Error};
use log::{debug, info, warn};
use serde::{Deserialize, Serialize};
use crate::jobs::{Installable, InstallReader, InstallWriter, JobCacheReader, JobCacheWriter, JobEnvironment};

#[derive(Serialize, Deserialize, Clone, Eq, PartialEq)]
pub struct FileJob {
    file: String,
    location: String,

    root: Option<bool>,
    link: Option<bool>
}

#[typetag::serde(name = "file")]
impl Installable for FileJob {

    fn install(&self, env: &JobEnvironment, writer: &mut InstallWriter) -> anyhow::Result<()> {
        let root = self.root.unwrap_or(false);

        // Get source file
        let mut file = env.module_path.clone();
        file.push(&self.file);
        if !file.exists() { return Err(Error::msg(format!("File ('{}') does not exist", file.to_string_lossy()))); }

        // Get target location
        let mut target = PathBuf::from(shellexpand::tilde(&self.location).as_ref());

        if target.exists() {
            // There is already a file at the target location
            info!("Caching and removing current file");
            writer.cache.cache_foreign(&target, "original");
            env.shell.remove(&target, root, None).context("Failed to remove original file to replace")?;

        } else if let Some(path) = target.parent() {
            if path.file_name().is_none() {
                // Do nothing, is a relative path to running directory
                // See https://github.com/rust-lang/rust/issues/36861
            } else if !path.exists() {
                // Parent dir does not yet exist
                info!("Making parent directory");
                env.shell.make_dir(path, root, None).context("Failed to make parent directories")?;

            } else if !path.is_dir() {
                // Parent dir is not a file
                return Err(Error::msg("Location parent directory is not a directory"))
            }
        }

        // Link or Copy file
        if self.link.unwrap_or(false) {
            info!("Linking file to target location");
            env.shell.link(&file, &target, root, None).context("Failed to create symlink")?;
        } else {
            // TODO: process variables
            info!("Copying file to target location");
            env.shell.copy(&file, &target, root, None).context("Failed to copy file")?;
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
        info!("Removing file at target location");
        env.shell.remove(&target, self.root.unwrap_or(false), None).context("Failed to remove installed file")?;

        if let Some(original) = reader.cache.retrieve("original") {
            // Restore original file
            info!("Restoring original file");
            env.shell.copy(&original, &target, self.root.unwrap_or(false), None).context("Failed to restore original file")?;
        }

        Ok(())
    }

    fn update(&self, old: &dyn Installable, env: &JobEnvironment, writer: &mut InstallWriter, reader: &InstallReader) -> Option<anyhow::Result<()>> {
        let old = old.as_any().downcast_ref::<Self>()?;

        // Uninstall old file if necessary
        if self.location != old.location {
            old.uninstall(env, reader).unwrap_or_else(|e| warn!("{e}"));
        }
        
        // Install new file
        if let Err(e) = self.install(env, writer) {
            return Some(Err(e));
        }
        
        // Make sure that the original file is not overwritten if updating at the same location
        if self.location == old.location {
            writer.cache.undo_cache("original");
        }

        Some(Ok(()))
    }

    fn construct_title(&self) -> String {
        let action = self.link.unwrap_or(false);
        format!("{} the file '{}' to its target location",
            if action { "Linking" } else { "Copying" },
            &self.file
        )
    }
}