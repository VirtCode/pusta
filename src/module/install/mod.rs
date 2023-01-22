pub mod shell;
pub(crate) mod neoshell;

use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use anyhow::{Context, Error};
use log::{error, info, warn};
use serde::{Deserialize, Serialize};
use crate::jobs::{InstallReader, InstallWriter, Job, JobEnvironment};
use crate::jobs::cache::{JobCacheReader, JobCacheWriter};
use crate::jobs::resources::{JobResources, ResourceFile};
use crate::module::install::neoshell::Shell;
use crate::module::Module;
use crate::output;
use crate::registry::cache::Cache;
use serde_with::{serde_as, TimestampMilliSeconds};
use serde_with::formats::Flexible;
use colored::Colorize;

#[serde_as]
#[derive(Serialize, Deserialize, Clone)]
pub struct InstalledModule {
    pub module: Module,
    pub data: Vec<JobData>,
    #[serde_as(as = "TimestampMilliSeconds<String, Flexible>")]
    pub installed: SystemTime,
    #[serde_as(as = "TimestampMilliSeconds<String, Flexible>")]
    pub updated: SystemTime
}

#[derive(Default, Serialize, Deserialize, Clone)]
pub struct JobData {
    success: bool,
    resources: Vec<ResourceFile>
}

pub struct Installer {
    shell: Shell
}

impl Installer {

    pub fn new(shell: Shell) -> Self {
        return Installer {
            shell
        }
    }

    pub fn install(&self, module: Module, cache_handler: &Cache) -> Option<InstalledModule> {

        // Create environment
        let env = JobEnvironment {
            shell: &self.shell,
            module: module.qualifier.unique().to_owned(),
            module_path: module.path.clone(),
        };

        let cache = match cache_handler.create_module_cache(&module) {
            Ok(c) => c,
            Err(e) => {
                error!("Failed to determine job cache ({}), install cannot continue", e.to_string());
                return None;
            }
        };

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

            let result = Installer::install_job(job, &env, &cache);
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

        let result = InstalledModule {
            module,
            data,
            installed: SystemTime::now(),
            updated: SystemTime::now(),
        };

        // Uninstall on failure
        if failure && output::prompt_yn("Undo the already taken actions now?", true) {

            self.uninstall(&result, cache_handler);

            None
        } else {
            Some(result)
        }
    }

    pub fn uninstall(&self, module: &InstalledModule, cache_handler: &Cache) {
        // Create environment
        let env = JobEnvironment {
            shell: &self.shell,
            module: module.module.qualifier.unique().to_owned(),
            module_path: module.module.path.clone(),
        };

        let cache = match cache_handler.create_module_cache(&module.module) {
            Ok(c) => c,
            Err(e) => {
                error!("Failed to determine job cache for uninstall ({}), uninstall will probably fail", e.to_string());
                PathBuf::from(format!("/tmp/pusta/{}", module.module.qualifier.unique())) // tmp as fallback
            }
        };

        // Go through every job with its data in reverse order
        let mut clean = true;
        for (i, (job, data)) in module.module.jobs.iter().zip(&module.data).enumerate().rev() {

            // Skip unsuccessful jobs
            if !data.success { continue }

            // Create cache dir
            let mut cache = cache.clone();
            cache.push(i.to_string());

            if !Installer::uninstall_job(job, &env, &cache) {
                clean = false;
            }
        }

        if !clean {
            warn!("Not all jobs could be undone correctly, system may be polluted");
        }

        fs::remove_dir_all(&cache).unwrap_or_else(|e| error!("Failed to remove installed cache, future installs may fail: {e}"));
    }

    pub fn install_job(job: &Job, env: &JobEnvironment, cache: &Path) -> JobData {
        info!("{}...", job.title().bright_white());

        // Prepare writer
        let mut writer = InstallWriter {
            cache: JobCacheWriter::start(),
            resources: JobResources::new()
        };

        // Perform install
        let success = if let Err(e) = job.install(env, &mut writer) {
            error!("Failed to do job: {e}");
            false
        } else { true };

        // Manage job data
        writer.cache.end(cache);
        let resources = writer.resources.process(&env.module_path);


        JobData {
            success, resources
        }
    }

    pub fn uninstall_job(job: &Job, env: &JobEnvironment, cache: &Path) -> bool {
        info!("Undoing \"{}\"...", job.title().bright_white());

        // Prepare reader
        let reader = InstallReader {
            cache: JobCacheReader::open(cache)
        };

        // Perform uninstall
        if let Err(e) = job.uninstall(env, &reader) {
            error!("Failed to undo job: {e}"); false
        } else { true }
    }
}