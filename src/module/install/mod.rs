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

#[derive(Serialize, Deserialize, Clone)]
pub struct InstalledModule {
    pub module: Module,
    pub data: Vec<JobData>,
    pub installed: SystemTime,
    pub updated: SystemTime
}

#[derive(Default, Serialize, Deserialize, Clone)]
struct JobData {
    success: bool,
    resources: Vec<ResourceFile>
}

struct Installer {
    shell: Shell,

}

impl Installer {
    fn cache_path(module: &Module, path: &Path) -> PathBuf {
        let mut cache = path.to_owned();
        cache.push(module.qualifier.unique().replace("/", "-"));
        cache
    }

    pub fn install(&self, module: Module, cache_base: &Path) -> Option<InstalledModule> {

        // Create environment
        let env = JobEnvironment {
            shell: &self.shell,
            module: module.qualifier.unique().to_owned(),
            module_path: module.path.clone(),
        };

        let cache = Installer::cache_path(&module, cache_base);

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

            self.uninstall(&result, cache_base);

            None
        } else {
            Some(result)
        }
    }

    pub fn uninstall(&self, module: &InstalledModule, cache: &Path) {
        // Create environment
        let env = JobEnvironment {
            shell: &self.shell,
            module: module.module.qualifier.unique().to_owned(),
            module_path: module.module.path.clone(),
        };

        let cache = Installer::cache_path(&module.module, cache);

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