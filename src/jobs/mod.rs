mod types;
mod cache;

use std::fs;
use std::os::unix::raw::time_t;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use log::{error, warn};
use crate::module::install::neoshell::Shell;
use serde::{Deserialize, Serialize};
use crate::jobs::cache::{JobCacheReader, JobCacheWriter};
use crate::jobs::types::Installable;

/// This is the environment provided to every installable
pub struct JobEnvironment {
    /// Abstraction over the system's shell
    pub shell: Shell,

    pub module: String,
    pub module_path: PathBuf
}

/// This struct represents a job which can be specified to be installed for a module
#[derive(Serialize, Deserialize)]
pub struct Job {
    /// Title of the job, if none, one will be generated
    title: Option<String>,
    /// Whether a job is optional, meaning failure will not cancel the whole installation
    optional: Option<bool>,

    /// The actual function of the job
    job: Box<dyn Installable>
}

impl Job {

    /// Returns the title of the job
    pub fn title(&self) -> String {
        self.title.unwrap_or_else(|| self.job.construct_title()).clone()
    }

    /// Returns whether the job is optional
    pub fn optional(&self) -> bool {
        self.optional.unwrap_or(false)
    }

    /// Installs the job
    pub fn install(&self, env: &JobEnvironment, cache: &mut JobCacheWriter) -> anyhow::Result<()> {
        self.job.install(env, cache)
    }

    /// Uninstalls the job
    pub fn uninstall(&self, env: &JobEnvironment, cache: &JobCacheReader) -> anyhow::Result<()> {
        self.job.uninstall(env, cache)
    }
}