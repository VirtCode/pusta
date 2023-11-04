use std::cmp;
use std::time::SystemTime;
use anyhow::anyhow;
use log::{debug, error, info};
use serde::{Deserialize, Serialize};
use crate::config::ConfigPackage;
use crate::jobs::{BuiltJob, Job, JobEnvironment, JobError};
use crate::module::installed::InstalledModule;
use crate::module::Module;
use crate::module::repository::Repository;
use crate::variables::{generate_magic, merge_variables, Variable};

pub(super) struct ModuleInstructions {
    pub new: Option<BuiltModule>,
    pub old: Option<BuiltModule>,

    pub apply: Vec<bool>,
    pub revert: Vec<bool>,
}

#[derive(Serialize, Deserialize)]
pub struct BuiltModule {
    pub jobs: Vec<BuiltJob>,
    pub used_variables: Variable
}

pub struct ModuleEnvironment {
    pub(crate) magic_variables: Variable,
    pub(crate) system_variables: Variable,
    pub(crate) package_config: ConfigPackage
}

/// builds a module install
pub(super) fn install(module: &Module, repository: &Repository, env: &ModuleEnvironment) -> anyhow::Result<ModuleInstructions> {
    info!("Building module {}", module.qualifier.unique());

    let variables = merge_variables(&module.variables.unwrap_or_else(|| Variable::base()),
                                    &repository.variables.unwrap_or_else(|| Variable::base()),
                                    &env.system_variables, &env.magic_variables);

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
        new: Some(BuiltModule { jobs: built, used_variables: variables }),
        old: None
    })
}

/// builds a module removal
pub(super) fn remove(module: InstalledModule) -> anyhow::Result<ModuleInstructions> {
    Ok(ModuleInstructions {
        apply: vec![],
        revert: vec![true; module.built.len()],
        new: None,
        old: Some(module.built)
    })
}

/// builds a module update
pub(super) fn update(installed: InstalledModule, module: &Module, repository: &Repository, env: &ModuleEnvironment) -> anyhow::Result<ModuleInstructions>{

    // build variables and env
    let variables = merge_variables(&module.variables.unwrap_or_else(|| Variable::base()),
                                    &repository.variables.unwrap_or_else(|| Variable::base()),
                                    &env.system_variables, &env.magic_variables);

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
    for jobs in module.jobs.iter()
        .map(Some).chain(vec![None; cmp::max(0, installed.module.jobs.len() as i32 - module.jobs.len() as i32) as usize])
        .zip(
            installed.module.jobs.iter().zip(installed.built.jobs.iter()).enumerate()
                .map(Some).chain(vec![None; cmp::max(0, module.jobs.len() as i32 - installed.module.jobs.len() as i32) as usize])
        ) {

        match jobs {
            // job has changed
            (Some(new), Some((index, (old, old_built)))) => {
                if new == old &&
                    old_built.resources.iter().any(|i| i.changed(&job_env.path)) &&
                    old_built.change_variables(&installed.built.used_variables, &variables) {

                    // copy job and skip
                    built.push((*old_built).clone());
                    apply.push(false);
                } else {

                    // partial build is possible
                    if let Some(result) = new.partial(old, old_built, &job_env) {
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
        new: Some(BuiltModule { jobs: built, used_variables: variables }),
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
            error!("Could not work with file '{file}': {error}");
        }
        JobError::Other(message, error) => {
            error!("{message}: {error}");
        }
    }

    anyhow!("failed to build module")
}