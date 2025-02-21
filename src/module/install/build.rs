use std::cmp;
use std::time::SystemTime;
use anyhow::anyhow;
use log::{debug, error, info};
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, TimestampMilliSeconds};
use serde_with::formats::Flexible;
use crate::config::ConfigPackage;
use crate::jobs::{BuiltJob, Job, JobEnvironment, JobError};
use crate::module::install::InstalledModule;
use crate::module::Module;
use crate::module::repository::Repository;
use crate::variables::{generate_magic, merge_variables, Variable};

pub(super) struct ModuleInstructions {
    pub new: Option<BuiltModule>,
    pub old: Option<BuiltModule>,

    pub apply: Vec<bool>,
    pub revert: Vec<bool>,
}

#[serde_as]
#[derive(Serialize, Deserialize, Clone)]
pub struct BuiltModule {
    pub jobs: Vec<BuiltJob>,
    pub used_variables: Variable,

    #[serde_as(as = "TimestampMilliSeconds<String, Flexible>")]
    pub time: SystemTime,
}

impl BuiltModule {
    fn new(jobs: Vec<BuiltJob>, used_variables: Variable) -> Self {
        Self {
            jobs, used_variables,
            time: SystemTime::now()
        }
    }
}

pub struct ModuleEnvironment {
    pub magic_variables: Variable,
    pub system_variables: Variable,
    pub injected_variables: Variable,
    pub package_config: ConfigPackage
}

/// builds a module install
pub(super) fn install(module: &Module, repository: &Repository, env: &ModuleEnvironment) -> anyhow::Result<ModuleInstructions> {
    info!("Building module {} for installation", module.qualifier.unique());

    let empty = Variable::base();
    let variables = merge_variables(module.variables.as_ref().unwrap_or_else(|| &empty),
                                    repository.load_variables()?.as_ref().unwrap_or_else(|| &empty),
                                    &env.injected_variables, &env.system_variables, &env.magic_variables);

    let job_env = JobEnvironment {
        variables: &variables,
        path: module.path.clone(),
        package_config: env.package_config.clone()
    };

    let mut built = vec![];

    for job in &module.jobs {
        debug!("Building job '{}'", job.title());
        built.push(job.build(&job_env).map_err(|e| handle_build_error(&module, e))?);
    }

    Ok(ModuleInstructions {
        apply: vec![true; built.len()],
        revert: vec![],
        new: Some(BuiltModule::new(built, variables)),
        old: None
    })
}

/// builds a module removal
pub(super) fn remove(module: InstalledModule) -> anyhow::Result<ModuleInstructions> {
    info!("Collecting module {} for removal", module.module.qualifier.unique());
    Ok(ModuleInstructions {
        apply: vec![],
        revert: vec![true; module.built.jobs.len()],
        new: None,
        old: Some(module.built)
    })
}

/// builds a module update
pub(super) fn update(installed: InstalledModule, module: &Module, repository: &Repository, env: &ModuleEnvironment) -> anyhow::Result<ModuleInstructions>{
    info!("Building update for module {}", module.qualifier.unique());

    // build variables and env
    let empty = Variable::base();
    let variables = merge_variables(module.variables.as_ref().unwrap_or_else(|| &empty),
                                    repository.load_variables()?.as_ref().unwrap_or_else(|| &empty),
                                    &env.injected_variables, &env.system_variables, &env.magic_variables);

    let job_env = JobEnvironment {
        variables: &variables,
        path: module.path.clone(),
        package_config: env.package_config.clone()
    };

    // create trackers for diff
    let mut built = vec![]; // built jobs
    let mut apply = vec![]; // built to apply
    let mut revert = vec![false; installed.built.jobs.len()]; // old built to revert

    // loop through jobs and check for updates
    // create a zipped iterator with two options to accommodate for smaller and bigger job arrays
    let mut new_iter = module.jobs.iter();
    let mut old_iter = installed.module.jobs.iter().zip(installed.built.jobs.iter()).enumerate();

    for jobs in std::iter::from_fn(|| {
        let new = new_iter.next();
        let old = old_iter.next();

        if new.is_none() && old.is_none() { None }
        else { Some((new, old))}
    }) {

        match jobs {
            // job has changed
            (Some(new), Some((index, (old, old_built)))) => {
                if new == old &&
                    !old_built.resources.iter().any(|i| i.changed(&job_env.path)) &&
                    !old_built.change_variables(&installed.built.used_variables, &variables) {

                    // copy job and skip
                    built.push(old_built.clone());
                    apply.push(false);
                } else {

                    // partial build is possible
                    if let Some(result) = new.partial(old, &old_built, &job_env) {
                        let result = result.map_err(|e| handle_build_error(&module, e))?;

                        // save partial job to be run
                        built.push(result);
                        apply.push(true);

                    } else {
                        // save revert old job and new job
                        revert[index] = true;
                        built.push(new.build(&job_env).map_err(|e| handle_build_error(&module, e))?);
                        apply.push(true);
                    }
                }
            }
            // new job appeared
            (Some(new), None) => {
                built.push(new.build(&job_env).map_err(|e| handle_build_error(&module, e))?);
                apply.push(true);
            }
            // old job was removed
            (None, Some((index, _))) => {
                revert[index] = true;
            }
            _ => { unreachable!() }
        }
    }

    Ok(ModuleInstructions {
        apply, revert,
        new: Some(BuiltModule::new(built, variables)),
        old: Some(installed.built)
    })
}

fn handle_build_error(module: &Module, error: JobError) -> anyhow::Error {
    error!("Failed to build module {}:", module.qualifier.unique());

    match error {
        JobError::Variable(error, source, file) => {
            error.print(&*file.to_string_lossy(), &source);
        }
        JobError::Resources(file, error) => {
            error!("Could not work with file '{}': {error}", file.to_string_lossy());
        }
        JobError::Other(message, error) => {
            error!("{message}: {error}");
        }
    }

    anyhow!("failed to build module")
}
