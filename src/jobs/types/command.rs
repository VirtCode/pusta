use anyhow::Context;
use log::info;
use crate::jobs::{Installable, JobCacheReader, JobCacheWriter, JobEnvironment};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct CommandJob {
    install: String,
    uninstall: Option<String>,

    show_output: Option<bool>,
    root: Option<bool>
}

#[typetag::serde(name = "script")]
impl Installable for CommandJob {

    fn install(&self, env: &JobEnvironment, cache: &mut JobCacheWriter) -> anyhow::Result<()> {

        info!("Running command '{}'", &self.install);
        env.shell.run(&self.install, self.root.unwrap_or(false), self.show_output.unwrap_or(true)).context("Failed to run custom command")?;

        Ok(())
    }

    fn uninstall(&self, env: &JobEnvironment, cache: &JobCacheReader) -> anyhow::Result<()> {

        if let Some(uninstall) = &self.uninstall {
            info!("Running uninstaller command '{}'", uninstall);
            env.shell.run(uninstall, self.root.unwrap_or(false), self.show_output.unwrap_or(true)).context("Failed to run custom uninstall command")?;
        }

        Ok(())
    }

    fn construct_title(&self) -> String {
        "run a custom set command".to_owned()
    }
}