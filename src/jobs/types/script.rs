use anyhow::{Context, Error};
use log::info;
use serde::{Deserialize, Serialize};
use crate::jobs::{Installable, InstallReader, InstallWriter, JobCacheReader, JobCacheWriter, JobEnvironment};

#[derive(Serialize, Deserialize, Clone, Eq, PartialEq)]
pub struct ScriptJob {
    install: String,
    uninstall: Option<String>,

    show_output: Option<bool>,
    root: Option<bool>
}

#[typetag::serde(name = "script")]
impl Installable for ScriptJob {

    fn install(&self, env: &JobEnvironment, writer: &mut InstallWriter, update: bool) -> anyhow::Result<()> {

        // Create path to script
        let mut path = env.module_path.clone();
        path.push(&self.install);
        if !path.exists() { return Err(Error::msg(format!("Script ('{}') does not exist", path.to_string_lossy()))) }

        info!("Launching install script file");
        // Prepare and run script (unchecked because in own directory)
        env.shell.unchecked.make_executable(&path, false).context("Failed to make script executable")?; // no root because the file is in the pusta repo
        // TODO: Process variables
        env.shell.run_script(&path, self.root.unwrap_or(false), self.show_output.unwrap_or(true)).context("Script execution failed")?;

        // Cache uninstall file
        if let Some(uninstall) = &self.uninstall {
            writer.cache.cache_own(env, uninstall, "uninstall");
        }

        // Mark installed file as resource
        writer.resources.mark(self.install.clone());

        Ok(())
    }

    fn uninstall(&self, env: &JobEnvironment, reader: &InstallReader) -> anyhow::Result<()> {

        // Run uninstaller if present
        if let Some(uninstall) = reader.cache.retrieve("uninstall") {
            info!("Launching uninstaller script file");
            env.shell.run_script(&uninstall, self.root.unwrap_or(false), self.show_output.unwrap_or(true)).context("Failed to run uninstaller script")?;
        }

        Ok(())
    }

    fn construct_title(&self) -> String {
        format!("Running the install script {}", self.install)
    }
}