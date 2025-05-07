use dyn_eq::DynEq;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use crate::jobs::{BuiltJob, Installable, JobEnvironment, JobResult};
use crate::module::change::RunChange;

/// This job installs a package from the system
#[derive(Serialize, Deserialize, Clone, Eq, PartialEq, JsonSchema)]
pub struct PackageJob {
    names: String
}

impl PackageJob {
    fn name_vec(&self) -> Vec<String> {
        self.names.split(' ').map(|s| s.to_owned()).collect()
    }
}

#[typetag::serde(name = "package")]
impl Installable for PackageJob {

    fn build(&self, env: &JobEnvironment) -> JobResult<BuiltJob> {
        let mut built = BuiltJob::new();

        let names = self.name_vec();
        built.change(Box::new(RunChange::new(
            env.package_config.create_install(&names),
            Some(env.package_config.create_remove(&names)),
            env.path.clone(), true, false)));

        built.root = env.package_config.root;

        Ok(built)
    }

    fn partial(&self, old: &dyn Installable, previous: &BuiltJob, env: &JobEnvironment) -> Option<JobResult<BuiltJob>> {
        let old = old.as_any().downcast_ref::<Self>()?;

        // Compare packages
        let old = old.name_vec();
        let new = self.name_vec();

        let remove: Vec<String> = old.iter().filter(|s| !new.contains(*s)).cloned().collect();
        let install: Vec<String> = new.iter().filter(|s| !old.contains(*s)).cloned().collect();

        let mut built = BuiltJob::new();

        // remove removed modules
        if !remove.is_empty() {
            built.change(Box::new(RunChange::new(
                env.package_config.create_remove(&remove),
                None, env.path.clone(), true, false)));
        }

        // install new modules
        built.change(Box::new(RunChange::new(
            env.package_config.create_install(&install),
            Some(env.package_config.create_remove(&new)),
            env.path.clone(), true, false)));

        built.root = env.package_config.root;

        Some(Ok(built))
    }

    fn construct_title(&self) -> String {
        format!("Installing the package(s) '{}' on the system", self.name_vec().join("', '"))
    }
}
