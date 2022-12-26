mod package;
mod file;

use std::fs;
use std::os::unix::raw::time_t;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use log::{error, warn};
use crate::module::install::neoshell::Shell;
use serde::{Deserialize, Serialize};

/// This is the environment provided to every installable
pub struct JobEnvironment {
    /// Abstraction over the system's shell
    pub shell: Shell,

    pub module: String,
    pub module_path: PathBuf
}

/// This trait will specify a job procedure type used by a Job
#[typetag::serde(tag = "type")]
pub trait Installable {
    /// Installs the procedure with a given environment
    fn install(&self, env: &JobEnvironment, cache: &mut JobCacheWriter) -> anyhow::Result<()>;
    /// Uninstalls the given procedure with a given environment
    fn uninstall(&self, env: &JobEnvironment, cache: &JobCacheReader) -> anyhow::Result<()>;

    /// Invents a completely new title if none is provided
    fn construct_title(&self) -> String;
}

/// This struct represents a job which can be specified to be installed for a module
#[derive(Serialize, Deserialize)]
pub struct Job {
    /// Title of the job, if none, one will be generated
    title: Option<String>,
    /// Whether a job is optional, meaning failure will not cancel the whole installation
    optional: Option<bool>,

    /// The actual function of the job
    job: Box<dyn Installable>
}

impl Job {

    /// Returns the title of the job
    pub fn title(&self) -> String {
        self.title.unwrap_or_else(|| self.job.construct_title()).clone()
    }

    /// Returns whether the job is optional
    pub fn optional(&self) -> bool {
        self.optional.unwrap_or(false)
    }

    /// Installs the job
    pub fn install(&self, env: &JobEnvironment, cache: &mut JobCacheWriter) -> anyhow::Result<()> {
        self.job.install(env, cache)
    }

    /// Uninstalls the job
    pub fn uninstall(&self, env: &JobEnvironment, cache: &JobCacheReader) -> anyhow::Result<()> {
        self.job.uninstall(env, cache)
    }
}

/// This struct is used by individual jobs so that they can cache data
pub struct JobCacheWriter {
    temp: PathBuf,
    files: Vec<(String, PathBuf)>
}

impl JobCacheWriter {

    /// Starts the begin of the cache, by creating its temporary folder to cache foreign files
    pub fn start() -> Self {
        // Generate a new unique temp directory
        let temp = PathBuf::from(format!("/tmp/pusta/{}/", SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis()));

        if let Err(e) = fs::create_dir_all(&temp) {
            warn!("Failed to create cache tmp directory: {e}")
        }

        Self {
            temp,
            files: vec![]
        }
    }

    /// Marks a file that is part of the module as to be cached under a given alias
    pub fn cache_own(&mut self, env: &JobEnvironment, name: &str, alias: &str) {
        let mut target = env.module_path.clone();
        target.push(name);

        if target.exists() {
            self.files.push((alias.to_owned(), target));
        } else {
            warn!("Could not cache own file {name} since it does not exist");
        }
    }

    /// Marks a file that is anywhere as to be cached under a given alias.
    /// Since pusta cannot (and actively does not) ensure that that file remains untouched until the cache is ran, the file ist first copied to a temporary folder
    pub fn cache_foreign(&mut self, path: &Path, alias: &str) {
        if !path.exists() {
            warn!("Could not cache foreign file at {} since it does not exist", path.to_string_lossy());
            return;
        }

        let mut temp = self.temp.clone();
        temp.push(alias);

        if let Err(e) = fs::copy(path, &temp) {
            warn!("Failed to cache foreign file at {}: {e}", path.to_string_lossy());
        } else {
            self.files.push((alias.to_owned(), temp));
        }
    }

    /// Collects the marked files to a cache location
    pub fn end(&self, target: &Path) {
        for (alias, path) in self.files {
            let mut location = target.to_owned();
            location.push(alias);

            if let Err(e) = fs::copy(path, location) {
                error!("Failed to end cache on file {}: {e}", path.to_string_lossy());
            }
        }

        if let Err(e) = fs::remove_dir_all(&self.temp) {
            warn!("Failed to remove temporary cache directory: {e}");
        }
    }
}

/// This struct reads a cache folder
pub struct JobCacheReader {
    cache: PathBuf
}

impl JobCacheReader {

    /// Opens a cache folder and checks whether it exists
    pub fn open(path: &Path) -> Self {

        if !path.exists() {
            warn!("Opened nonexistent cache, errors may follow")
        }

        Self {
            cache: path.to_owned()
        }
    }

    /// Tries to retrieve a file based on its alias, returning a path to it if it exists
    pub fn retrieve(&self, alias: &str) -> Option<PathBuf> {
        let mut path = self.cache.clone();
        path.push(alias);

        if path.exists() {
           Some(path)
        } else {
            warn!("Couldn't find cached file '{alias}' in cache");
            None
        }
    }

    /// Removes the cached data from disk
    pub fn delete(&self) {
        if let Err(e) = fs::remove_dir_all(&self.cache) {
            warn!("Failed to remove job cache: {e}");
        }
    }
}