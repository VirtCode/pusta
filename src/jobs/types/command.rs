use anyhow::Context;
use log::info;
use crate::jobs::{Installable, InstallReader, InstallWriter, JobCacheReader, JobCacheWriter, JobEnvironment};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
pub struct CommandJob {
    install: String,
    uninstall: Option<String>,

    show_output: Option<bool>,
    root: Option<bool>
}

#[typetag::serde(name = "script")]
impl Installable for CommandJob {

    fn install(&self, env: &JobEnvironment, writer: &mut InstallWriter) -> anyhow::Result<()> {

        env.shell.run_command(&self.install, self.root.unwrap_or(false), self.show_output.unwrap_or(true)).context("Failed to run custom command")?;

        Ok(())
    }

    fn uninstall(&self, env: &JobEnvironment, reader: &InstallReader) -> anyhow::Result<()> {

        if let Some(uninstall) = &self.uninstall {
            env.shell.run_command(uninstall, self.root.unwrap_or(false), self.show_output.unwrap_or(true)).context("Failed to run custom uninstall command")?;
        }

        Ok(())
    }

    fn construct_title(&self) -> String {
        "Running a custom command".to_owned()
    }
}