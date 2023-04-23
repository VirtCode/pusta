use std::any::Any;
use dyn_clone::{clone_trait_object, DynClone};
use dyn_eq::{DynEq, eq_trait_object};
use crate::jobs::cache::{JobCacheReader, JobCacheWriter};
use crate::jobs::{InstallReader, InstallWriter, JobEnvironment};

mod package;
mod file;
mod script;
mod command;

// Has to be cloned during the install process creating a new installed module and also needs to be compared
clone_trait_object!(Installable);
eq_trait_object!(Installable);

/// This trait will specify a job procedure type used by a Job
#[typetag::serde(tag = "type")]
pub trait Installable: DynClone + DynEq + Any {
    /// Installs the procedure with a given environment
    fn install(&self, env: &JobEnvironment, writer: &mut InstallWriter) -> anyhow::Result<()>;
    /// Uninstalls the given procedure with a given environment
    fn uninstall(&self, env: &JobEnvironment, reader: &InstallReader) -> anyhow::Result<()>;
    /// Installs the job as an update, given the old installable
    fn update(&self, old: &dyn Installable, env: &JobEnvironment, writer: &mut InstallWriter, reader: &InstallReader) -> Option<anyhow::Result<()>>;

    /// Invents a completely new title if none is provided
    fn construct_title(&self) -> String;
}