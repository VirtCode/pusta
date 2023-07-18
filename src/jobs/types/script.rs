use std::path::PathBuf;
use anyhow::{Context, Error};
use log::{info, warn};
use serde::{Deserialize, Serialize};
use crate::jobs::{Installable, InstallReader, InstallWriter, JobCacheReader, JobCacheWriter, JobEnvironment};

#[derive(Serialize, Deserialize, Clone, Eq, PartialEq)]
pub struct ScriptJob {
    install: String,
    uninstall: Option<String>,

    reinstall: Option<bool>,
    show_output: Option<bool>,
    root: Option<bool>,
    running_directory: Option<String>
}

#[typetag::serde(name = "script")]
impl Installable for ScriptJob {

    fn install(&self, env: &JobEnvironment, writer: &mut InstallWriter) -> anyhow::Result<()> {

        // Create path to script
        let mut path = env.module_path.clone();
        path.push(&self.install);
        if !path.exists() { return Err(Error::msg(format!("Script ('{}') does not exist", path.to_string_lossy()))) }

        let mut running_directory = env.module_path.clone();
        if let Some(path) = self.running_directory.as_ref() { running_directory.push(shellexpand::tilde(path).as_ref()) }

        info!("Launching install script file");
        // Prepare and run script (unchecked because in own directory)
        env.shell.unchecked.make_executable(&path, false, None).context("Failed to make script executable")?; // no root and running directory because the file is in the pusta repo
        // TODO: Process variables
        env.shell.run_script(&path, self.root.unwrap_or(false), self.show_output.unwrap_or(true), Some(&running_directory)).context("Script execution failed")?;

        // Cache uninstall file
        if let Some(uninstall) = &self.uninstall {
            writer.cache.cache_own(env, uninstall, "uninstall");
        }

        // Mark installed file as resource
        writer.resources.mark(self.install.clone());

        Ok(())
    }

    fn uninstall(&self, env: &JobEnvironment, reader: &InstallReader) -> anyhow::Result<()> {

        let mut running_directory = env.module_path.clone();
        if let Some(path) = self.running_directory.as_ref() { running_directory.push(shellexpand::tilde(path).as_ref()) }

        // Run uninstaller if present
        if let Some(uninstall) = reader.cache.retrieve("uninstall") {
            info!("Launching uninstaller script file");
            env.shell.run_script(&uninstall, self.root.unwrap_or(false), self.show_output.unwrap_or(true), Some(&running_directory)).context("Failed to run uninstaller script")?;
        }

        Ok(())
    }

    fn update(&self, old: &dyn Installable, env: &JobEnvironment, writer: &mut InstallWriter, reader: &InstallReader) -> Option<anyhow::Result<()>> {
        let old = old.as_any().downcast_ref::<Self>()?;

        if self.reinstall.unwrap_or_default() {
           self.uninstall(env, reader).unwrap_or_else(|e| warn!("{e}"));
        }
        
        Some(self.install(env, writer))
    }

    fn construct_title(&self) -> String {
        format!("Running the install script {}", self.install)
    }
}