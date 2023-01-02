use crate::jobs::cache::{JobCacheReader, JobCacheWriter};
use crate::jobs::{InstallReader, InstallWriter, JobEnvironment};

mod package;
mod file;
mod script;
mod command;

/// This trait will specify a job procedure type used by a Job
#[typetag::serde(tag = "type")]
pub trait Installable {
    /// Installs the procedure with a given environment
    fn install(&self, env: &JobEnvironment, writer: &mut InstallWriter) -> anyhow::Result<()>;
    /// Uninstalls the given procedure with a given environment
    fn uninstall(&self, env: &JobEnvironment, reader: &InstallReader) -> anyhow::Result<()>;

    /// Invents a completely new title if none is provided
    fn construct_title(&self) -> String;
}