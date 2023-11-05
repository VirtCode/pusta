use std::path::Path;
use serde::{Deserialize, Serialize};
use crate::jobs::{BuiltJob, Installable, JobEnvironment, JobResult};
use crate::jobs::helper::{process_variables, resource_load};
use crate::module::change::ScriptChange;

#[derive(Serialize, Deserialize, Clone, Eq, PartialEq)]
pub struct ScriptJob {
    install: String,
    uninstall: Option<String>,

    reinstall: Option<bool>,
    show_output: Option<bool>,
    root: Option<bool>,
    running_directory: Option<String>
}

#[typetag::serde(name = "script")]
impl Installable for ScriptJob {
    fn build(&self, env: &JobEnvironment) -> JobResult<BuiltJob> {
        let mut built = BuiltJob::new();

        // calculate running directory
        let mut running_directory = env.path.clone();
        if let Some(path) = self.running_directory.as_ref() {
            running_directory.push(shellexpand::tilde(path).as_ref());
        }

        // process install script
        let install = {
            let path = Path::new(&self.install);

            let install = resource_load(path, env, &mut built)?;
            process_variables(&install, path, env, &mut built)?
        };

        // process uninstall script
        let uninstall = if let Some(uninstall_path) = &self.uninstall {
            let path = Path::new(&self.install);

            let uninstall = resource_load(path, env, &mut built)?;
            Some(process_variables(&uninstall, path, env, &mut built)?)
        } else { None };

        built.change(Box::new(ScriptChange::new(install, uninstall, running_directory, self.show_output.unwrap_or(true))));

        built.root = self.root.unwrap_or_default();

        Ok(built)
    }

    fn partial(&self, old: &dyn Installable, previous: &BuiltJob, env: &JobEnvironment) -> Option<JobResult<BuiltJob>> {
        let old = old.as_any().downcast_ref::<Self>()?;

        // force full reinstall if last job was set to do it
        if old.reinstall.unwrap_or_default() {
            return None;
        }

        // just build the install otherwise
        Some(self.build(env))
    }

    fn construct_title(&self) -> String {
        format!("Running the install script {}", self.install)
    }
}