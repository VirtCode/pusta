use std::{env, fs, io};
use std::os::unix::fs::symlink;
use std::os::unix::raw::time_t;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::SystemTime;
use chksum::hash::SHA1;
use fs_extra::dir::CopyOptions;
use log::{debug, info};
use serde::{Deserialize, Serialize};
use crate::module::transaction::shell;

/// Represents an atomic change
#[typetag::serde(tag = "type")]
pub trait AtomicChange {
    /// Applies the atomic change
    fn apply(&mut self, runtime: &ChangeRuntime) -> ChangeResult;
    /// Reverts the atomic change
    fn revert(&mut self, runtime: &ChangeRuntime) -> ChangeResult;
}

const TEMP_PATH: &str = "temp";
const TEMP_CACHE: &str = "cache";

pub struct ChangeRuntime {
    dir: PathBuf
}

impl ChangeRuntime {
    /// Caches a file for later restoration
    fn cache(&self, path: &Path) -> Result<PathBuf, ChangeError> {
        // calculate hash for target location
        let result = chksum::hash::hash::<SHA1, _>(path.to_string_lossy().to_string());

        // create path
        let mut target = self.dir.clone();
        target.push(TEMP_CACHE);
        target.push(result.to_hex_lowercase());

        fs_extra::copy_items(&[path], &target, &CopyOptions::default())
            .map_err(|e| ChangeError::cache(path.to_owned(), target.clone(), e.to_string()))?;

        Ok(target)
    }

    /// Stores a string in a temporary file
    fn temp(&self, src: &str) -> Result<PathBuf, ChangeError> {
        // calculate hash for storage location
        let result = chksum::hash::hash::<SHA1, _>(src);

        // create path
        let mut path = self.dir.clone();
        path.push(TEMP_PATH);
        path.push(result.to_hex_lowercase());

        // save file
        fs_extra::file::write_all(&path, src)
            .map_err(|e| ChangeError::temp(src.to_owned(), path.clone(), e.to_string()))?;

        Ok(path)
    }
}

pub enum ChangeError {
    Filesystem {
        path: PathBuf,
        message: String,
        cause: String
    },
    Cache {
        path: PathBuf,
        target_path: PathBuf,
        message: String
    },
    Temp {
        content: String,
        target_path: PathBuf,
        message: String
    },
    CommandFatal {
        command: String,
        message: String
    },
    Command {
        command: String,
        output: String,
        error: String,
        exit_code: i32 // i32::MAX if the process was terminated by signal
    },
    ScriptFatal {
        script: String,
        message: String
    },
    Script {
        script: String,
        output: String,
        error: String,
        exit_code: i32 // i32::MAX if the script was terminated by signal
    }
}

pub type ChangeResult = Result<(), ChangeError>;

impl ChangeError {
    fn filesystem(path: PathBuf, message: String, cause: String) -> Self {
        Self::Filesystem { path, message, cause }
    }

    fn command(command: String, output: String, error: String, exit_code: i32) -> Self {
        Self::Command { command, output, error, exit_code }
    }
    fn script(script: String, output: String, error: String, exit_code: i32) -> Self {
        Self::Script { script, output, error, exit_code }
    }
    fn command_fatal(command: String, message: String) -> Self {
        Self::CommandFatal { command, message }
    }

    fn script_fatal(script: String, message: String) -> Self {
        Self::ScriptFatal { script, message }
    }

    fn temp(content: String, target_path: PathBuf, message: String) -> Self {
        Self::Temp { content, target_path, message }
    }

    fn cache(path: PathBuf, target_path: PathBuf, message: String) -> Self {
        Self::Cache { path, target_path, message }
    }
}


/// This change cleans the spot where a file is going to be put
#[derive(Serialize, Deserialize)]
struct ClearChange {
    /// File to clear
    file: PathBuf,

    /// Cache where the cleared file will be stored
    cache: Option<PathBuf>
}

impl ClearChange {
    pub fn new(file: PathBuf) -> Self {
        Self {
            file,
            cache: None
        }
    }
}

#[typetag::serde]
impl AtomicChange for ClearChange {
    fn apply(&mut self, runtime: &ChangeRuntime) -> ChangeResult {

        /// Only cache the file if it exists
        if self.file.exists() {
            self.cache = Some(runtime.cache(&self.file)?);

            fs_extra::remove_items(&[&self.file])
                .map_err(|e| ChangeError::filesystem(self.file.clone(), "failed to delete original file or directory".into(), e.to_string()))?;
        }

        Ok(())
    }

    fn revert(&mut self, runtime: &ChangeRuntime) -> ChangeResult {

        // Only undo cache if it was cached
        if let Some(cached) = &self.cache {
            fs_extra::copy_items(&[&cached], &self.file, &CopyOptions::default())
                .map_err(|e| ChangeError::filesystem(self.file.clone(), "failed to restore original file".into(), e.to_string()))?;
        }

        Ok(())
    }
}

/// This change inserts some text into a file somewhere
#[derive(Serialize, Deserialize)]
struct WriteChange {
    /// Text to insert into a file
    text: String,
    /// File to insert text into
    file: PathBuf,
}

impl WriteChange {
    pub fn new(text: String, file: PathBuf) -> Self {
        Self { text, file }
    }
}

#[typetag::serde]
impl AtomicChange for WriteChange {
    fn apply(&mut self, runtime: &ChangeRuntime) -> Result<(), ChangeError> {
        // Write the file
        fs_extra::file::write_all(&self.file, &self.text)
            .map_err(|e| ChangeError::filesystem(self.file.clone(), "failed to write to file".into(), e.to_string()))
    }

    fn revert(&mut self, runtime: &ChangeRuntime) -> Result<(), ChangeError> {
        // Delete the file
        fs_extra::remove_items(&[&self.file])
            .map_err(|e| ChangeError::filesystem(self.file.clone(), "failed to delete file".into(), e.to_string()))
    }
}

/// This change copies a file somewhere
#[derive(Serialize, Deserialize)]
struct CopyChange {
    /// File to copy to
    file: PathBuf,
    /// Source file to copy
    source: PathBuf
}

impl CopyChange {
    pub fn new(file: PathBuf, source: PathBuf) -> Self {
        Self { file, source }
    }
}

#[typetag::serde]
impl AtomicChange for CopyChange {
    fn apply(&mut self, runtime: &ChangeRuntime) -> ChangeResult {
        // Copy files
        fs_extra::copy_items(&[&self.source], &self.file, &CopyOptions::default())
            .map_err(|e| ChangeError::filesystem(self.file.clone(), "failed to copy file or directory to that location".into(), e.to_string()))?;

        Ok(())
    }

    fn revert(&mut self, runtime: &ChangeRuntime) -> ChangeResult {
        // Delete copied files
        fs_extra::remove_items(&[&self.file])
            .map_err(|e| ChangeError::filesystem(self.file.clone(), "failed to remove copied file or directory".into(), e.to_string()))
    }
}

/// This change links a file to a location
#[derive(Serialize, Deserialize)]
struct LinkChange {
    /// File to place link at
    file: PathBuf,
    /// File to link
    source: PathBuf
}

impl LinkChange {
    pub fn new(file: PathBuf, source: PathBuf) -> Self {
        Self { file, source }
    }
}

#[typetag::serde]
impl AtomicChange for LinkChange {
    fn apply(&mut self, runtime: &ChangeRuntime) -> ChangeResult {
        // Link files
        symlink(&self.source, &self.file)
            .map_err(|e| ChangeError::filesystem(self.file.clone(), "failed to create symlink there".into(), e.to_string()))
    }

    fn revert(&mut self, runtime: &ChangeRuntime) -> ChangeResult {
        // Delete symlink
        fs::remove_file(&self.file)
            .map_err(|e| ChangeError::filesystem(self.file.clone(), "failed to remove symlink".into(), e.to_string()))
    }
}

/// This change runs a command on the shell
#[derive(Serialize, Deserialize)]
struct RunChange {
    /// Command to run when applying the change
    apply: String,
    /// Command to run when reverting the change
    revert: Option<String>,

    /// Running directory
    dir: PathBuf,

    /// Whether the command should print output and the user should be able to interact with it
    interactive: bool
}

impl RunChange {
    pub fn new(apply: String, revert: Option<String>, dir: PathBuf, interactive: bool) -> Self {
        Self { apply, revert, dir, interactive }
    }
}

#[typetag::serde]
impl AtomicChange for RunChange {
    fn apply(&mut self, runtime: &ChangeRuntime) -> ChangeResult {
        // Run command on shell
        let result = shell::run_command(&self.apply, &self.dir, self.interactive)
            .map_err(|e| ChangeError::command_fatal(self.apply.clone(), e))?;

        // Check result
        if !result.status.success() {
            return Err(ChangeError::command(self.apply.clone(), result.stdout, result.stderr, result.status.code().unwrap_or(i32::MAX)))
        }

        Ok(())
    }

    fn revert(&mut self, runtime: &ChangeRuntime) -> ChangeResult {
        // only revert if revert command is set
        if let Some(revert) = &self.revert {
            // Run command on shell
            let result = shell::run_command(&revert, &self.dir, self.interactive)
                .map_err(|e| ChangeError::command_fatal(revert.clone(), e))?;

            // Check result
            if !result.status.success() {
                return Err(ChangeError::command(revert.clone(), result.stdout, result.stderr, result.status.code().unwrap_or(i32::MAX)))
            }
        }

        Ok(())
    }
}

/// This change runs a command on the shell
#[derive(Serialize, Deserialize)]
struct ScriptChange {
    /// Script code to run when applying the change
    apply: String,
    /// Script code to run when reverting the change
    revert: Option<String>,

    /// Running directory where the script is ran
    dir: PathBuf,

    /// Whether the command should print output and the user should be able to interact with it
    interactive: bool
}

impl ScriptChange {
    pub fn new(apply: String, revert: Option<String>, dir: PathBuf, interactive: bool) -> Self {
        Self { apply, revert, dir, interactive }
    }
}

#[typetag::serde]
impl AtomicChange for ScriptChange {
    fn apply(&mut self, runtime: &ChangeRuntime) -> ChangeResult {
        // Store file on disk
        let file = runtime.temp(&self.apply)?;

        // Run command on shell
        let result = shell::run_script(&file, &self.dir, self.interactive)
            .map_err(|e| ChangeError::script_fatal(self.apply.clone(), e))?;

        // Check result
        if !result.status.success() {
            return Err(ChangeError::script(self.apply.clone(), result.stdout, result.stderr, result.status.code().unwrap_or(i32::MAX)))
        }

        Ok(())
    }

    fn revert(&mut self, runtime: &ChangeRuntime) -> ChangeResult {
        // only revert if revert script is set
        if let Some(revert) = &self.revert {
            // Store file on disk
            let file = runtime.temp(&revert)?;

            // Run command on shell
            let result = shell::run_script(&file, &self.dir, self.interactive)
                .map_err(|e| ChangeError::script_fatal(revert.clone(), e))?;

            // Check result
            if !result.status.success() {
                return Err(ChangeError::script(revert.clone(), result.stdout, result.stderr, result.status.code().unwrap_or(i32::MAX)))
            }
        }

        Ok(())
    }
}
