mod shell;
pub mod worker;

use std::fs;
use std::fs::Permissions;
use std::os::unix::fs::{PermissionsExt, symlink};
use std::path::{Path, PathBuf};
use chksum::hash::SHA1;
use dyn_clone::{clone_trait_object, DynClone};
use fs_extra::dir::CopyOptions;
use serde::{Deserialize, Serialize};

clone_trait_object!(AtomicChange);

/// Represents an atomic change
#[typetag::serde(tag = "type")]
pub trait AtomicChange: DynClone {
    /// Applies the atomic change
    fn apply(&self, runtime: &ChangeRuntime) -> ChangeResult;

    /// Reverts the atomic change
    fn revert(&self, runtime: &ChangeRuntime) -> ChangeResult;

    /// Returns a description of the change, based on its data
    fn describe(&self) -> String;

    /// Returns a list of critical data used
    fn files(&self) -> Vec<(String, String)>;
}

const TEMP_PATH: &str = "temp";
const TEMP_CACHE: &str = "cache";

pub struct ChangeRuntime {
    pub cache: PathBuf,
    pub temp: PathBuf
}

impl ChangeRuntime {
    /// Caches a file for later restoration
    fn cache_save(&self, path: &Path) -> Result<(), ChangeError> {
        let target = self.cache_dir(path);

        // Skip symlinks, because they cannot be copied
        if path.is_symlink() {
            return Ok(())
        }

        copy(path, &target)
            .map_err(|e| ChangeError::cache(path.to_owned(), target, e.to_string()))?;

        Ok(())
    }

    fn cache_load(&self, path: &Path) -> Option<PathBuf> {
        let target = self.cache_dir(path);

        if target.exists() { Some(target) }
        else { None }
    }

    /// Creates a path reference for an inherit cache
    fn cache_dir(&self, path: &Path) -> PathBuf {
        // calculate hash for target location
        let result = chksum::hash::hash::<SHA1, _>(path.to_string_lossy().to_string());

        // create path
        let mut target = self.cache.clone();
        target.push(result.to_hex_lowercase());

        target
    }

    /// Stores a string in a temporary file
    fn temp(&self, src: &str) -> Result<PathBuf, ChangeError> {
        // calculate hash for storage location
        let result = chksum::hash::hash::<SHA1, _>(src);

        // create path
        let mut path = self.temp.clone();
        fs::create_dir_all(&path)
            .map_err(|e| ChangeError::temp(src.to_owned(), path.clone(), e.to_string()))?;

        path.push(result.to_hex_lowercase());

        // save file
        fs_extra::file::write_all(&path, src)
            .map_err(|e| ChangeError::temp(src.to_owned(), path.clone(), e.to_string()))?;

        Ok(path)
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub enum ChangeError {
    WorkerFatal {
        message: String
    },
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

    pub fn fatal(message: String) -> Self {
        Self::WorkerFatal { message }
    }
}

impl ToString for ChangeError {
    fn to_string(&self) -> String {
        match self {
            ChangeError::WorkerFatal { message } => {
                format!("Fatal error occurred: {message}")
            }
            ChangeError::Filesystem { message, cause, path } => {
                format!("Could not use filesystem at '{}', {message}, caused by: {cause}", path.to_string_lossy())
            }
            ChangeError::Cache { path, target_path, message } => {
                format!("Could not cache file '{}' to '{}', because of: {message}", path.to_string_lossy(), target_path.to_string_lossy())
            }
            ChangeError::Temp { content, target_path, message } => {
                format!("Could not store file temporarily at '{}', because of {message}", target_path.to_string_lossy())
            }
            ChangeError::CommandFatal { command, message } => {
                format!("Fatal error occurred when running command: {message}\nCommand run was '{command}'")
            }
            ChangeError::Command { command, output, error, exit_code } => {
                format!("Command did not run successfully and exited with code {exit_code}\nCommand was: {command}\nConsole output was:\n{output}\nError output was:\n{error}")
            }
            ChangeError::ScriptFatal { script, message } => {
                format!("Fatal error occurred when running script: {message}")
            }
            ChangeError::Script { script, output, error, exit_code } => {
                format!("Script did not run successfully and exited with code {exit_code}\nScript was:\n{script}\nConsole output was:\n{output}\nError output was:\n{error}")
            }
        }
    }
}


/// This change cleans the spot where a file is going to be put
#[derive(Serialize, Deserialize, Clone)]
pub struct ClearChange {
    /// File to clear
    file: PathBuf,

    /// Whether to inherit cache from previous runs
    inherit: bool,
}

impl ClearChange {
    pub fn new(file: PathBuf, inherit: bool) -> Self {
        Self { file, inherit, }
    }
}

#[typetag::serde]
impl AtomicChange for ClearChange {
    fn apply(&self, runtime: &ChangeRuntime) -> ChangeResult {
        // Skip everything if inheriting cache from parent
        if !self.inherit {

            // Only cache the file if it exists, create parent dir otherwise
            if self.file.exists() {
                runtime.cache_save(&self.file)?;

                fs_extra::remove_items(&[&self.file])
                    .map_err(|e| ChangeError::filesystem(self.file.clone(), "failed to delete original file or directory".into(), e.to_string()))?;
            } else if let Some(parent) = self.file.parent() {

                fs::create_dir_all(parent)
                    .map_err(|e| ChangeError::filesystem(self.file.clone(), "failed to create parent directory".into(), e.to_string()))?;
            }
        }

        Ok(())
    }

    fn revert(&self, runtime: &ChangeRuntime) -> ChangeResult {

        // Only undo cache if it was cached
        if let Some(cached) = runtime.cache_load(&self.file) {
            copy(&cached, &self.file)
                .map_err(|e| ChangeError::filesystem(self.file.clone(), "failed to restore original file".into(), e.to_string()))?;
        }

        Ok(())
    }

    fn describe(&self) -> String {
        format!("cleans out the directory '{}'", &self.file.to_string_lossy())
    }

    fn files(&self) -> Vec<(String, String)> {
        vec![]
    }
}

/// This change creates a directory
#[derive(Serialize, Deserialize, Clone)]
pub struct DirectoryChange {
    /// Directory to create
    directory: PathBuf
}

impl DirectoryChange {
    pub fn new(directory: PathBuf) -> Self {
        Self { directory }
    }
}

#[typetag::serde]
impl AtomicChange for DirectoryChange {
    fn apply(&self, _runtime: &ChangeRuntime) -> ChangeResult {
        fs::create_dir_all(&self.directory)
            .map_err(|e| ChangeError::filesystem(self.directory.clone(), "failed to create directory".into(), e.to_string()))?;

        Ok(())
    }

    fn revert(&self, _runtime: &ChangeRuntime) -> ChangeResult {
        Ok(())
    }

    fn describe(&self) -> String {
        format!("creating the directory if it doesn't exist '{}'", &self.directory.to_string_lossy())
    }

    fn files(&self) -> Vec<(String, String)> {
        vec![]
    }
}


/// This change inserts some text into a file somewhere
#[derive(Serialize, Deserialize, Clone)]
pub struct WriteChange {
    /// Text to insert into a file
    text: String,
    /// Permission mode on the created file
    #[serde(default = "WriteChange::permissions_default")]
    permissions: u32,
    /// File to insert text into
    file: PathBuf,
}

impl WriteChange {
    pub fn new(text: String, permissions: u32, file: PathBuf) -> Self {
        Self { text, permissions, file }
    }

    pub fn permissions_default() -> u32 {
        // for backwards compatibility
        0o0644
    }
}

#[typetag::serde]
impl AtomicChange for WriteChange {
    fn apply(&self, runtime: &ChangeRuntime) -> Result<(), ChangeError> {
        // Write the file
        fs_extra::file::write_all(&self.file, &self.text)
            .map_err(|e| ChangeError::filesystem(self.file.clone(), "failed to write to file".into(), e.to_string()))?;

        // Set permissions
        fs::set_permissions(&self.file, Permissions::from_mode(self.permissions))
            .map_err(|e| ChangeError::filesystem(self.file.clone(), "failed to change permissions of created file".into(), e.to_string()))
    }

    fn revert(&self, runtime: &ChangeRuntime) -> Result<(), ChangeError> {
        // Delete the file
        fs_extra::remove_items(&[&self.file])
            .map_err(|e| ChangeError::filesystem(self.file.clone(), "failed to delete file".into(), e.to_string()))
    }

    fn describe(&self) -> String {
        format!("writes a file to '{}'", &self.file.to_string_lossy())
    }

    fn files(&self) -> Vec<(String, String)> {
        vec![("content".to_string(), self.text.clone())]
    }
}

/// This change copies a file somewhere
#[derive(Serialize, Deserialize, Clone)]
pub struct CopyChange {
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
    fn apply(&self, runtime: &ChangeRuntime) -> ChangeResult {
        // Copy files
        copy(&self.source, &self.file)
            .map_err(|e| ChangeError::filesystem(self.file.clone(), "failed to copy file or directory to that location".into(), e.to_string()))?;

        Ok(())
    }

    fn revert(&self, runtime: &ChangeRuntime) -> ChangeResult {
        // Delete copied files
        fs_extra::remove_items(&[&self.file])
            .map_err(|e| ChangeError::filesystem(self.file.clone(), "failed to remove copied file or directory".into(), e.to_string()))
    }

    fn describe(&self) -> String {
        format!("copies the file file '{}' to '{}'", &self.source.to_string_lossy(), &self.file.to_string_lossy())
    }

    fn files(&self) -> Vec<(String, String)> {
        vec![]
    }
}

/// This change links a file to a location
#[derive(Serialize, Deserialize, Clone)]
pub struct LinkChange {
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
    fn apply(&self, runtime: &ChangeRuntime) -> ChangeResult {
        // Delete old file if it exists
        if self.file.exists() {
            fs::remove_file(&self.file)
                .map_err(|e| ChangeError::filesystem(self.file.clone(), "failed to remove file to create symlink".into(), e.to_string()))?;
        }

        // Link files
        symlink(&self.source, &self.file)
            .map_err(|e| ChangeError::filesystem(self.file.clone(), "failed to create symlink there".into(), e.to_string()))
    }

    fn revert(&self, runtime: &ChangeRuntime) -> ChangeResult {
        // Delete symlink
        fs::remove_file(&self.file)
            .map_err(|e| ChangeError::filesystem(self.file.clone(), "failed to remove symlink".into(), e.to_string()))
    }

    fn describe(&self) -> String {
        format!("links the file file '{}' to '{}'", &self.source.to_string_lossy(), &self.file.to_string_lossy())
    }

    fn files(&self) -> Vec<(String, String)> {
        vec![]
    }
}

/// This change runs a command on the shell
#[derive(Serialize, Deserialize, Clone)]
pub struct RunChange {
    /// Command to run when applying the change
    apply: String,
    /// Command to run when reverting the change
    revert: Option<String>,

    /// Running directory
    dir: PathBuf,

    /// Whether the command should print output and the user should be able to interact with it
    interactive: bool,
    /// Whether the command is allowed to fail
    #[serde(default)]
    failable: bool
}

impl RunChange {
    pub fn new(apply: String, revert: Option<String>, dir: PathBuf, interactive: bool, failable: bool) -> Self {
        Self { apply, revert, dir, interactive, failable }
    }
}

#[typetag::serde]
impl AtomicChange for RunChange {
    fn apply(&self, runtime: &ChangeRuntime) -> ChangeResult {
        // Run command on shell
        let result = shell::run_command(&self.apply, &self.dir, self.interactive)
            .map_err(|e| ChangeError::command_fatal(self.apply.clone(), e))?;

        // Check result
        if !self.failable && !result.status.success() {
            return Err(ChangeError::command(self.apply.clone(), result.stdout, result.stderr, result.status.code().unwrap_or(i32::MAX)))
        }

        Ok(())
    }

    fn revert(&self, runtime: &ChangeRuntime) -> ChangeResult {
        // only revert if revert command is set
        if let Some(revert) = &self.revert {
            // Run command on shell
            let result = shell::run_command(&revert, &self.dir, self.interactive)
                .map_err(|e| ChangeError::command_fatal(revert.clone(), e))?;

            // Check result
            if !self.failable && !result.status.success() {
                return Err(ChangeError::command(revert.clone(), result.stdout, result.stderr, result.status.code().unwrap_or(i32::MAX)))
            }
        }

        Ok(())
    }

    fn describe(&self) -> String {
        format!("runs a{} command on the shell in '{}'", if self.interactive { "n interactive" } else {""}, self.dir.to_string_lossy())
    }

    fn files(&self) -> Vec<(String, String)> {
        let mut vec = vec![("install".to_string(), self.apply.clone())];
        if let Some(revert) = &self.revert {
            vec.push(("uninstall".to_string(), revert.clone()));
        }
        vec
    }
}

/// This change runs a command on the shell
#[derive(Serialize, Deserialize, Clone)]
pub struct ScriptChange {
    /// Script code to run when applying the change
    apply: String,
    /// Script code to run when reverting the change
    revert: Option<String>,

    /// Running directory where the script is ran
    dir: PathBuf,

    /// Whether the command should print output and the user should be able to interact with it
    interactive: bool,
    /// Whether the script is allowed to fail
    #[serde(default)]
    failable: bool
}

impl ScriptChange {
    pub fn new(apply: String, revert: Option<String>, dir: PathBuf, interactive: bool, failable: bool) -> Self {
        Self { apply, revert, dir, interactive, failable }
    }
}

#[typetag::serde]
impl AtomicChange for ScriptChange {
    fn apply(&self, runtime: &ChangeRuntime) -> ChangeResult {
        // Store file on disk
        let file = runtime.temp(&self.apply)?;

        // Run command on shell
        let result = shell::run_script(&file, &self.dir, self.interactive)
            .map_err(|e| ChangeError::script_fatal(self.apply.clone(), e))?;

        // Check result
        if !self.failable && !result.status.success() {
            return Err(ChangeError::script(self.apply.clone(), result.stdout, result.stderr, result.status.code().unwrap_or(i32::MAX)))
        }

        Ok(())
    }

    fn revert(&self, runtime: &ChangeRuntime) -> ChangeResult {
        // only revert if revert script is set
        if let Some(revert) = &self.revert {
            // Store file on disk
            let file = runtime.temp(&revert)?;

            // Run command on shell
            let result = shell::run_script(&file, &self.dir, self.interactive)
                .map_err(|e| ChangeError::script_fatal(revert.clone(), e))?;

            // Check result
            if !self.failable && !result.status.success() {
                return Err(ChangeError::script(revert.clone(), result.stdout, result.stderr, result.status.code().unwrap_or(i32::MAX)))
            }
        }

        Ok(())
    }

    fn describe(&self) -> String {
        format!("runs a{} script in '{}'", if self.interactive { "n interactive" } else {""}, self.dir.to_string_lossy())
    }

    fn files(&self) -> Vec<(String, String)> {
        let mut vec = vec![("script".to_string(), self.apply.clone())];
        if let Some(revert) = &self.revert {
            vec.push(("revert script".to_string(), revert.clone()));
        }
        vec
    }
}

/// Copies either a file or directory
fn copy(from: &Path, to: &Path) -> fs_extra::error::Result<u64>{
    if from.is_dir() {
        let copy_options = CopyOptions::new().overwrite(true).copy_inside(true);
        fs_extra::dir::copy(from, to, &copy_options)
    } else {
        fs_extra::file::copy(from, to, &fs_extra::file::CopyOptions::default().overwrite(true))
    }
}
