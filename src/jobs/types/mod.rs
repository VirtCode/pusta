use crate::jobs::cache::{JobCacheReader, JobCacheWriter};
use crate::jobs::JobEnvironment;

mod package;
mod file;
mod script;
mod command;

/// This trait will specify a job procedure type used by a Job
#[typetag::serde(tag = "type")]
pub trait Installable {
    /// Installs the procedure with a given environment
    fn install(&self, env: &JobEnvironment, cache: &mut JobCacheWriter) -> anyhow::Result<()>;
    /// Uninstalls the given procedure with a given environment
    fn uninstall(&self, env: &JobEnvironment, cache: &JobCacheReader) -> anyhow::Result<()>;

    /// Invents a completely new title if none is provided
    fn construct_title(&self) -> String;
}