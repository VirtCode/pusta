use std::path::{PathBuf};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use crate::jobs::{BuiltJob, Installable, JobEnvironment, JobResult};
use crate::jobs::helper::{process_variables, resource_dir, resource_load, resource_mark};
use crate::module::change::{ClearChange, CopyChange, LinkChange, WriteChange};

#[derive(Serialize, Deserialize, Clone, Eq, PartialEq, JsonSchema)]
pub struct FileJob {
    file: String,
    location: String,
    permissions: Option<u32>,

    root: Option<bool>,
    link: Option<bool>
}

impl FileJob {

    /// Deploys the file to the optimal location
    fn deploy(&self, target: PathBuf, env: &JobEnvironment, built: &mut BuiltJob) -> JobResult<()>{
        // Get source file
        let source = PathBuf::from(&self.file);
        let is_dir = resource_dir(&source, env)?;

        // deploy file depending on link and dir status
        match (self.link.unwrap_or_default(), is_dir) {
            (false, false) => {
                let resource = resource_load(&source, env, built)?;
                let resource = process_variables(&resource, &source, env, built)?;

                built.change(Box::new(WriteChange::new(resource, self.permissions.unwrap_or(0o0644), target)));
            },
            (false, true) => {
                let path = resource_mark(&source, env, built)?;

                built.change(Box::new(CopyChange::new(target, path)));
            },
            (true, _) => {
                let path = resource_mark(&source, env, built)?;

                built.change(Box::new(LinkChange::new(target, path)));
            }
        }

        Ok(())
    }
}

#[typetag::serde(name = "file")]
impl Installable for FileJob {
    fn build(&self, env: &JobEnvironment) -> JobResult<BuiltJob> {
        let mut built = BuiltJob::new();

        // Get and prepare location
        let mut target = PathBuf::from(shellexpand::tilde(&self.location).as_ref());
        built.change(Box::new(ClearChange::new(target.clone(), false)));

        // deploy file to location
        self.deploy(target, env, &mut built)?;

        built.root = self.root.unwrap_or_default();

        Ok(built)
    }

    fn partial(&self, old: &dyn Installable, previous: &BuiltJob, env: &JobEnvironment) -> Option<JobResult<BuiltJob>> {
        let old = old.as_any().downcast_ref::<Self>()?;

        // reinstall whole if location changed
        if self.location != old.location {
            return None;
        }

        // update installation
        let mut built = BuiltJob::new();

        // prepare location but keep cache
        let target = PathBuf::from(shellexpand::tilde(&self.location).as_ref());
        built.change(Box::new(ClearChange::new(target.clone(), true)));

        // deploy file to location
        if let Err(e) = self.deploy(target, env, &mut built) {
            return Some(Err(e))
        }

        built.root = self.root.unwrap_or_default();

        Some(Ok(built))
    }

    fn construct_title(&self) -> String {
        let action = self.link.unwrap_or(false);
        format!("{} the file '{}' to its target location",
                if action { "Linking" } else { "Copying" },
                &self.file
        )
    }
}