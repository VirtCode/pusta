use std::path::Path;
use crate::jobs::{BuiltJob, Installable, JobEnvironment, JobError, JobResult};
use serde::{Deserialize, Serialize};
use crate::jobs::helper::process_variables;
use crate::module::change::RunChange;

#[derive(Serialize, Deserialize, Clone, Eq, PartialEq)]
pub struct CommandJob {
    install: String,
    uninstall: Option<String>,

    reinstall: Option<bool>,
    show_output: Option<bool>,
    root: Option<bool>,
    running_directory: Option<String>
}

#[typetag::serde(name = "command")]
impl Installable for CommandJob {

    fn build(&self, env: &JobEnvironment) -> JobResult<BuiltJob> {
        let mut built = BuiltJob::new();

        // calculate running directory
        let mut running_directory = env.path.clone();
        if let Some(path) = self.running_directory.as_ref() {
            running_directory.push(shellexpand::tilde(path).as_ref());
        }

        // create commands
        let install = process_variables(&self.install, Path::new("install-command"), env, &mut built)?;
        let uninstall = if let Some(uninstall) = &self.uninstall {
            Some(process_variables(uninstall, Path::new("uninstall-command"), env, &mut built)?)
        } else { None };

        // add change
        built.change(Box::new(RunChange::new(install, uninstall, running_directory, self.show_output.unwrap_or(true))));

        // set settings
        built.root = self.root.unwrap_or_default();

        Ok(built)
    }

    fn partial(&self, old: &dyn Installable, previous: &BuiltJob, env: &JobEnvironment) -> Option<Result<BuiltJob, JobError>> {
        let old = old.as_any().downcast_ref::<Self>()?;

        // force full reinstall if last job was set to do it
        if old.reinstall.unwrap_or_default() {
            return None;
        }

        // just build new job
        Some(self.build(env))
    }

    fn construct_title(&self) -> String {
        "Running a custom command".to_owned()
    }
}