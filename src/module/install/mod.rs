pub mod shell;
pub mod checked;

use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use anyhow::{anyhow, Context, Error};
use log::{debug, error, info, warn};
use serde::{Deserialize, Serialize};
use crate::jobs::{InstallReader, InstallWriter, Job, JobEnvironment};
use crate::jobs::cache::{JobCacheReader, JobCacheWriter};
use crate::jobs::resources::{JobResources, ResourceFile};
use crate::module::install::shell::Shell;
use crate::module::Module;
use crate::output;
use crate::registry::cache::Cache;
use serde_with::{serde_as, TimestampMilliSeconds};
use serde_with::formats::Flexible;
use colored::Colorize;
use crate::module::install::checked::CheckedShell;
use crate::output::prompt_yn;

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
    shell: CheckedShell
}

impl Installer {

    pub fn new(shell: CheckedShell) -> Self {
        Installer {
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
        let mut failure: i32 = -1;
        for (i, job) in module.jobs.iter().enumerate() {
            // If failed before, do not install next one
            if failure != -1 {
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
                } else if i == module.jobs.len() - 1 || !output::prompt_yn("The last job failed to complete, continue anyway?", false) {
                    failure = i as i32;
                }
            }
        }

        let result = InstalledModule {
            module,
            data,
            installed: SystemTime::now(),
            updated: SystemTime::now(),
        };


        if failure != -1 {
            error!("Not every job could be installed successfully");

            // First job failed, nothing to undo
            if failure == 0 {
                return None;
            }

            // Uninstall on failure
            if output::prompt_yn("Undo the already taken actions now?", true) {

                self.uninstall(&result, cache_handler);
                return None;
            }
        }

        Some(result)
    }

    pub fn update(&self, installed: &InstalledModule, module: Module, cache_handler: &Cache) -> Option<Option<InstalledModule>> {

        // Reinstall module if required
        if !installed.module.equals_jobs(&module) {
            info!("Module update contains changed job definitions, performing re-installation");
            return Some(self.reinstall(installed, module, cache_handler));
        }


        // Possibly migrate modules qualifier
        if installed.module.qualifier != module.qualifier {
            // Check that no cache is overwritten
            if cache_handler.has_module(&module.qualifier.unique()) {
                error!("Cannot migrate module qualifier to one that is already installed ({} -> {})", installed.module.qualifier.unique(), module.qualifier.unique());
                return None;
            }

            if let Err(e) = cache_handler.migrate_module_cache(&installed.module, &module) {
                warn!("Failed to migrate cache for new module qualifier ({} -> {}), some cache may be overwritten", installed.module.qualifier.unique(), module.qualifier.unique())
            }
        }

        // Update Jobs
        let env = JobEnvironment {
            shell: &self.shell,
            module: module.qualifier.unique().to_owned(),
            module_path: module.path.clone(),
        };

        let cache = match cache_handler.create_module_cache(&module) {
            Ok(c) => c,
            Err(e) => {
                error!("Failed to determine job cache ({}), update cannot continue", e.to_string());
                return None;
            }
        };

        let mut new_data = installed.data.clone();
        let mut smooth = true;

        for (i, (data, job)) in installed.data.iter()
            .zip(&installed.module.jobs).enumerate()
            .filter(|(_, (data, _))| {
                    data.resources.iter().any(|resource| !resource.up_to_date(&installed.module.path).unwrap_or(false)) ||
                    !data.success
            }) {

            // Prepare cache
            let mut cache = cache.clone();
            cache.push(i.to_string());

            // TODO: Use the update method every job provides. Also try to update jobs if job definitions have changed -> don't just reinstall
            let next_data = Installer::install_job(job, &env, &cache, true);
            if !next_data.success {
                if data.success {
                    warn!("Previously installed {}job failed to update correctly", if job.optional() { "optional " } else { "" });
                    smooth = false;
                } else {
                    warn!("Previously failed job install failed again")
                }
            }

            new_data[i] = next_data;
        }

        let installed = InstalledModule {
            module,
            data: new_data,
            installed: installed.installed.clone(),
            updated: SystemTime::now()
        };

        if !smooth {
            error!("Some jobs failed to update correctly, your install may be broken");

            if prompt_yn("Do you want to remove this whole module?", false) {
                self.uninstall(&installed, cache_handler);
                return Some(None);
            }
        }

        Some(Some(installed))
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

    pub fn reinstall(&self, installed: &InstalledModule, module: Module, cache_handler: &Cache) -> Option<InstalledModule> {

        info!("Performing uninstall...");
        self.uninstall(installed, cache_handler);

        info!("Performing install...");
        self.install(module, cache_handler)
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
            error!("{e}");
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