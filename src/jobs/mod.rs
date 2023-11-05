mod types;
mod helper;

use std::fs::File;
use std::io::Error;
use std::path::{Path, PathBuf};
use anyhow::Context;
use chksum::chksum;
use chksum::hash::SHA1;
use serde::{Deserialize, Serialize};
use crate::config::ConfigPackage;
use crate::jobs::types::Installable;
use crate::module::change::AtomicChange;
use crate::variables::{Variable, VariableError};
use crate::variables::evaluate::VariableEvalCounter;

/// This is the environment provided to every installable
pub struct JobEnvironment<'a> {
    pub variables: &'a Variable,
    pub path: PathBuf,
    pub package_config: ConfigPackage
}

/// this marks a resource used by the job
#[derive(Serialize, Deserialize, Clone)]
pub struct ResourceItem {
    /// relative path the file is located at
    path: PathBuf,
    /// checksum of installed version
    checksum: String
}

impl ResourceItem {
    /// creates the item and calculates the checksum
    pub fn create(path: PathBuf, parent: &Path) -> JobResult<Self> {
        let mut file = parent.to_owned();
        file.push(&path);

        let handle = File::open(file).map_err(|e| JobError::Other("could not open file to calculate checksum".into(), e.into()))?;
        let checksum = chksum::<SHA1, _>(handle).map_err(|e| JobError::Other("could not calculate checksum".into(), e.into()))?.to_hex_lowercase();

        Ok(Self { path, checksum })
    }

    /// checks the checksum compared to the new one, returns true if a change was detected
    pub fn changed(&self, parent: &Path) -> bool {
        let mut file = parent.to_owned();
        file.push(&self.path);

        File::open(file).context("failed to read file for checksum")
            .and_then(|f| chksum::<SHA1, _>(f).context("failed to calculate checksum"))
            .map(|c| c.to_hex_lowercase() != self.checksum)
            .unwrap_or(true)
    }

}

type JobResult<T> = Result<T, JobError>;

pub enum JobError {
    Variable(VariableError, String, PathBuf),
    Resources(PathBuf, Error),
    Other(String, anyhow::Error)
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

/// this struct contains all information about a built job
#[derive(Serialize, Deserialize, Clone)]
pub struct BuiltJob {
    /// title of the job
    pub title: String,

    /// is the job optional
    pub optional: bool,
    /// is the job to be run as root
    pub root: bool,

    /// changes to be made to the system
    pub changes: Vec<Box<dyn AtomicChange>>,

    /// resources on which the job depends
    pub resources: Vec<ResourceItem>,
    /// variables which were used during build
    pub variables: Vec<String>
}

impl BuiltJob {

    /// creates an empty build
    pub fn new() -> Self {
        Self {
            title: "unknown job".to_string(),
            optional: false,
            root: false,
            changes: vec![],
            resources: vec![],
            variables: vec![]
        }
    }

    /// adds a change to the build
    pub fn change(&mut self, change: Box<dyn AtomicChange>) {
        self.changes.push(change);
    }

    /// marks variables used by the job
    pub fn use_variables(&mut self, counter: VariableEvalCounter) {
        self.variables.append(&mut counter.usages());

        // deduplicate because same variables could've already been used by other resource
        self.variables.dedup();
    }

    /// marks a resource as used by the job
    pub fn mark_resource(&mut self, item: ResourceItem) {
        self.resources.push(item);
    }

    /// did variables change
    pub fn change_variables(&self, old: &Variable, new: &Variable) -> bool {
        self.variables.iter().any(|k| old.find(k) != new.find(k))
    }
}


