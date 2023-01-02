pub mod shell;
pub(crate) mod neoshell;

use std::fs;
use std::panic::resume_unwind;
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use anyhow::{Context, Error};
use log::{error, info, warn};
use serde::{Deserialize, Serialize};
use crate::jobs::{InstallWriter, Job, JobEnvironment};
use crate::jobs::cache::JobCacheWriter;
use crate::jobs::resources::{JobResources, ResourceFile};
use crate::module::install::neoshell::Shell;
use crate::module::Module;
use crate::output;

//TODO: A installer struct which takes a module, installs it, and returns an installed module, with all associated data

struct InstalledModule {
    module: Module,
    data: Vec<JobData>,
    installed: SystemTime,
    updated: SystemTime
}

#[derive(Default)]
struct JobData {
    success: bool,
    resources: Vec<ResourceFile>
}

struct Installer {
    shell: Shell,

}

impl Installer {
    pub fn install(&self, module: Module, cache: &Path) -> Option<InstalledModule> {

        // Create environment
        let env = JobEnvironment {
            shell: &self.shell,
            module: module.qualifier.unique().to_owned(),
            module_path: module.path.clone(),
        };

        let mut cache = cache.to_owned();
        cache.push(module.qualifier.unique().replace("/", "-"));

        let mut data = vec![];

        // Install every job
        let mut failure = false;
        for (i, job) in module.jobs.iter().enumerate() {
            // If failed before, do not install next one
            if failure {
                data.push(JobData::default());
                continue;
            }

            // Has not failed yet
            let mut cache = cache.clone();
            cache.push(i.to_string());

            let result = self.install_job(job, &env, &cache);
            let success = result.success;
            data.push(result);

            if !success {
                if job.optional() {
                    warn!("Continuing because the job is optional");
                } else if !output::prompt_yn("The last job failed to complete, continue anyway?", false) {
                    failure = true;
                }
            }
        }

        // Uninstall on failure
        if failure && output::prompt_yn("Undo the already taken actions now?", true) {
            // TODO: Reverse arrays
            // TODO: Basic Uninstall procedure
        }

        Some(InstalledModule {
            module,
            data,
            installed: SystemTime::now(),
            updated: SystemTime::now(),
        })
    }

    pub fn install_job(&self, job: &Job, env: &JobEnvironment, cache: &Path) -> JobData {

        // Prepare writer
        let mut writer = InstallWriter {
            cache: JobCacheWriter::start(),
            resources: JobResources::new()
        };

        // Perform install
        let success = if let Err(e) = job.install(env, &mut writer) {
            error!("Failed to install module: {e}");
            false
        } else { true };

        // Manage job data
        writer.cache.end(cache);
        let resources = writer.resources.process(&env.module_path);


        JobData {
            success, resources
        }
    }
}