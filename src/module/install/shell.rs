use std::env;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use anyhow::{Context, Error};
use log::{debug, info, warn};
use crate::config::{Config, ConfigSecurity, ConfigShell};
use crate::output;

const ROOT_COMMAND_KEY: &str = "%COMMAND%";
const PACKAGE_COMMAND_KEY: &str = "%PACKAGE%";
const FILE_PREVIEW_KEY: &str = "%FILE%";

pub struct Shell {
    shell_config: ConfigShell,
    extra_confirm: bool
}

impl Shell {
    pub fn new(config: &Config) -> Self {
        Shell {
            shell_config: config.system.clone(),
            extra_confirm: config.security.extra_confirm_everything
        }
    }

    /// Runs an arbitrary command on the user's shell
    pub fn run(&self, command: &str, root: bool, output: bool, running_directory: Option<&Path>) -> anyhow::Result<()> {
        let command = if root {
            self.shell_config.root_elevator.replace(ROOT_COMMAND_KEY, command)
        } else { command.to_string() };

        if self.extra_confirm && !output::prompt_yn(&format!("Really run '{}' on your shell?", &command), true) {
            return Err(Error::msg("User interjected shell command execution"));
        }

        let configured = self.shell_config.default_directory.as_ref().map(|s| PathBuf::from(shellexpand::tilde(s).as_ref()));
        let dir = running_directory.or(configured.as_deref());

        if !run(&command, output, dir)? {
            Err(Error::msg("Shell command did not succeed"))
        } else { Ok(()) }
    }

    /// Installs a package over the system package manager
    pub fn install(&self, packages: Vec<String>) -> anyhow::Result<()> {
        let packages = packages.join(" ");
        let command = self.shell_config.package_manager.install.clone().replace(PACKAGE_COMMAND_KEY, &packages);

        self.run(&command, self.shell_config.package_manager.root, true, None)
    }

    /// Uninstalls a package over the system package manager
    pub fn uninstall(&self, packages: Vec<String>) -> anyhow::Result<()> {
        let packages = packages.join(" ");
        let command = self.shell_config.package_manager.remove.clone().replace(PACKAGE_COMMAND_KEY, &packages);

        self.run(&command, self.shell_config.package_manager.root, true, None)
    }

    /// Makes all directories for the given path
    pub fn make_dir(&self, path: &Path, root: bool, running_directory: Option<&Path>) -> anyhow::Result<()> {
        let command = format!("mkdir -p {}", path.to_string_lossy());

        self.run(&command, root, false, running_directory)
    }

    /// Removes a file or directory at the given path
    pub fn remove(&self, path: &Path, root: bool, running_directory: Option<&Path>) -> anyhow::Result<()> {
        let command = format!("rm -r {}", path.to_string_lossy());

        self.run(&command, root, false, running_directory)
    }

    /// Copies a file or directory to a specific place
    pub fn copy(&self, source: &Path, destination: &Path, root: bool, running_directory: Option<&Path>) -> anyhow::Result<()> {
        let command = format!("cp -r {} {}", source.to_string_lossy(), destination.to_string_lossy());

        self.run(&command, root, false, running_directory)
    }

    /// Creates a symlink for a file or directory
    pub fn link(&self, source: &Path, destination: &Path, root: bool, running_directory: Option<&Path>) -> anyhow::Result<()> {
        let command = format!("ln -s {} {}", source.to_string_lossy(), destination.to_string_lossy());

        self.run(&command, root, false, running_directory)
    }

    /// Makes a given file executable
    pub fn make_executable(&self, path: &Path, root: bool, running_directory: Option<&Path>) -> anyhow::Result<()> {
        let command = format!("chmod +x {}", path.to_string_lossy());
        
        self.run(&command, root, false, running_directory)
    }

    /// Previews a file using utilities like less
    pub fn preview(&self, path: &Path, running_directory: Option<&Path>) -> anyhow::Result<()> {
        let command = self.shell_config.file_previewer.replace(FILE_PREVIEW_KEY, &path.to_string_lossy().to_string());

        self.run(&command, false, true, running_directory)
    }
}

const FALLBACK_SHELL: &str = "/bin/sh";

/// Runs a given command on the user's shell, with or without output
fn run(exec: &str, output: bool, running_directory: Option<&Path>) -> anyhow::Result<bool> {
    let shell = env::var("SHELL").unwrap_or_else(|_| { warn!("Current shell ($SHELL) is not defined, using {}", FALLBACK_SHELL); FALLBACK_SHELL.to_string()});

    // Set home as running directory and only push to it, eliminate behaviour based on command evocation dir
    let mut dir = PathBuf::from(env::var("HOME").context("No home directory ($HOME) defined, which is required")?);
    if let Some(running_directory) = running_directory { dir.push(running_directory) }

    debug!("{:?}", running_directory);

    let mut command = Command::new(&shell);
    if output { command.stdout(Stdio::inherit()).stdin(Stdio::inherit()).stderr(Stdio::inherit()); }
    command.current_dir(dir);
    command.arg("-c").arg(exec);

    debug!("Running shell command '{}' on {}", exec, shell);
    let mut result = command.spawn()?;

    Ok(result.wait()?.success())
}