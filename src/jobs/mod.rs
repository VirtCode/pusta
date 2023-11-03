mod types;
pub mod cache;
pub mod resources;

use std::fs;
use std::os::unix::raw::time_t;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use log::{error, warn};
use crate::module::install::shell::Shell;
use serde::{Deserialize, Serialize};
use crate::config::ConfigPackage;
use crate::jobs::cache::{CacheItem, JobCacheReader, JobCacheWriter};
use crate::jobs::resources::{JobResources, ResourceFile, ResourceItem};
use crate::jobs::types::Installable;
use crate::module::install::checked::CheckedShell;
use crate::module::transaction::change::AtomicChange;
use crate::variables::{Variable, VariableError};

/// This is the environment provided to every installable
pub struct JobEnvironment<'a> {
    pub variables: &'a Variable,
    pub path: PathBuf,
    pub package_config: ConfigPackage
}

/// This struct contains mechanisms used during installation
pub struct InstallWriter {
    pub cache: JobCacheWriter,
    pub resources: JobResources
}

pub struct InstallReader {
    pub cache: JobCacheReader
}

/// This struct represents a job which can be specified to be installed for a module
#[derive(Serialize, Deserialize, Clone, PartialEq, Eq)]
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
        self.title.clone().unwrap_or_else(|| self.job.construct_title())
    }

    /// Returns whether the job is optional
    pub fn optional(&self) -> bool {
        self.optional.unwrap_or(false)
    }

    /// Installs the job
    pub fn install(&self, env: &JobEnvironment, writer: &mut InstallWriter) -> anyhow::Result<()> {
        self.job.install(env, writer)
    }

    /// Uninstalls the job
    pub fn uninstall(&self, env: &JobEnvironment, reader: &InstallReader) -> anyhow::Result<()> {
        self.job.uninstall(env, reader)
    }

    pub fn update(&self, old: &Job, env: &JobEnvironment, writer: &mut InstallWriter, reader: &InstallReader) -> Option<anyhow::Result<()>> {
        self.job.update(old.job.as_ref(), env, writer, reader)
    }

    pub fn build(&self, env: &JobEnvironment) -> Result<BuiltJob, JobError> {
        let mut built = self.job.build(env)?;

        // change generic attributes
        built.optional = self.optional.unwrap_or(false);
        built.title = self.title();

        Ok(built)
    }

    pub fn partial(&self, old: &Job, previous: &BuiltJob, env: &JobEnvironment) -> Option<Result<BuiltJob, JobError>>{
        let mut built = self.job.partial(old.job.as_ref(), previous, env)?;

        if let Ok(job) = &mut built {
            job.title = self.title();
            job.optional = self.optional.unwrap_or(false); // TODO: Match with previous success, if succeeded, it is no longer optional
        }

        Some(built)
    }

}

pub struct BuiltJob {
    /// title of the job
    pub title: String,

    /// is the job optional
    pub optional: bool,
    /// is the job to be run as root
    pub root: bool,

    /// changes to be made to the system
    pub changes: Vec<Box<dyn AtomicChange>>,

    /// files to be cached beforehand
    pub caches: Vec<CacheItem>,
    /// resources on which the job depends
    pub resources: Vec<ResourceItem>,
    /// variables which were used during build
    pub variables: Vec<String>
}

type JobResult<T> = Result<T, JobError>;

enum JobError {
    Variable(VariableError),
    Resources(anyhow::Error)
}

impl BuiltJob {

    pub fn new() -> Self {
        Self {
            title: "unknown job".to_string(),
            optional: false,
            root: false,
            changes: vec![],
            caches: vec![],
            resources: vec![],
            variables: vec![]
        }
    }

    pub fn change(&mut self, change: Box<dyn AtomicChange>) {
        self.changes.push(change);
    }
}

// loads a resource from file to a string and throws an error if not found
pub fn load_resource(file: &Path, env: &JobEnvironment, built: &mut BuiltJob) -> JobResult<String> {
    Ok(String::new())
}

// checks a resource whether it is a file or not
pub fn check_resource(file: &Path, env: &JobEnvironment) -> JobResult<bool> {
    return Ok(false)
}

// checks that a resource exists and throws an error otherwise
pub fn mark_resource(file: &Path, env: &JobEnvironment, built: &mut BuiltJob) -> JobResult<()> {
    Ok(())
}

// processes the variables inside a given string, and throws an error if it could not be resolved
pub fn process_variables(string: &str, env: &JobEnvironment, built: &mut BuiltJob) -> JobResult<String> {
    Ok(string.to_owned())
}

