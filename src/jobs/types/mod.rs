use std::any::Any;
use dyn_clone::{clone_trait_object, DynClone};
use dyn_eq::{DynEq, eq_trait_object};
use schemars::JsonSchema;
use serde::Serialize;
use crate::jobs::{BuiltJob, JobEnvironment, JobResult};

pub mod package;
pub mod file;
pub mod script;
pub mod command;

#[allow(dead_code)]
#[derive(Serialize, JsonSchema)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum JobTypes {
    /// Package job
    Package(package::PackageJob),
    /// File job
    File(file::FileJob),
    /// Script job
    Script(script::ScriptJob),
    /// Command job
    Command(command::CommandJob)
}

// Has to be cloned during the install process creating a new installed module and also needs to be compared
clone_trait_object!(Installable);
eq_trait_object!(Installable);

/// This trait will specify a job procedure type used by a Job
#[typetag::serde(tag = "type")]
pub trait Installable: DynClone + DynEq + Any {
    /// builds the job into a built version of changes and resources etc.
    fn build(&self, env: &JobEnvironment) -> JobResult<BuiltJob>;
    /// builds the job into a built version, but with respecting the previous version
    fn partial(&self, old: &dyn Installable, previous: &BuiltJob, env: &JobEnvironment) -> Option<JobResult<BuiltJob>>;

    /// Invents a completely new title if none is provided
    fn construct_title(&self) -> String;
}