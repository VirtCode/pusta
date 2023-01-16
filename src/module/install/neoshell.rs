use std::env;
use std::path::Path;
use std::process::{Command, Stdio};
use anyhow::Error;
use log::{debug, warn};
use crate::config::{Config, ConfigSecurity, ConfigShell};
use crate::output;

const ROOT_COMMAND_KEY: &str = "$COMMAND$";
const PACKAGE_COMMAND_KEY: &str = "$PACKAGE$";
const FILE_PREVIEW_KEY: &str = "$FILE$";

pub struct Shell {
    shell_config: ConfigShell,
    security_config: ConfigSecurity
}

impl Shell {
    pub fn new(config: &Config) -> Self {
        Shell {
            shell_config: config.system.clone(),
            security_config: config.security.clone()
        }
    }

    /// Runs an arbitrary command on the user's shell
    pub fn run(&self, command: &str, root: bool, output: bool) -> anyhow::Result<()> {
        let command = if root {
            self.shell_config.root_elevator.replace(ROOT_COMMAND_KEY, command)
        } else { command.to_string() };

        if self.security_config.extra_confirm_everything && !output::prompt_yn(&format!("Really run '{}' on your shell?", &command), true) {
            return Err(Error::msg("User interjected shell command execution"));
        }

        if !run(&command, output)? {
            Err(Error::msg("Shell command did not succeed"))
        } else { Ok(()) }
    }

    /// Installs a package over the system package manager
    pub fn install(&self, packages: Vec<String>) -> anyhow::Result<()> {
        let packages = packages.join(" ");
        let command = self.shell_config.package_manager.install.clone().replace(PACKAGE_COMMAND_KEY, &packages);

        self.run(&command, self.shell_config.package_manager.root, true)
    }

    /// Uninstalls a package over the system package manager
    pub fn uninstall(&self, packages: Vec<String>) -> anyhow::Result<()> {
        let packages = packages.join(" ");
        let command = self.shell_config.package_manager.remove.clone().replace(PACKAGE_COMMAND_KEY, &packages);

        self.run(&command, self.shell_config.package_manager.root, true)
    }

    /// Makes all directories for the given path
    pub fn make_dir(&self, path: &Path, root: bool) -> anyhow::Result<()> {
        let command = format!("mkdir -p {}", path.canonicalize()?.to_string_lossy());

        self.run(&command, root, false)
    }

    /// Removes a file or directory at the given path
    pub fn remove(&self, path: &Path, root: bool) -> anyhow::Result<()> {
        let command = format!("rm -r {}", path.canonicalize()?.to_string_lossy());

        self.run(&command, root, false)
    }

    /// Copies a file or directory to a specific place
    pub fn copy(&self, source: &Path, destination: &Path, root: bool) -> anyhow::Result<()> {
        let command = format!("cp -r {} {}", source.canonicalize()?.to_string_lossy(), destination.canonicalize()?.to_string_lossy());

        self.run(&command, root, false)
    }

    /// Creates a symlink for a file or directory
    pub fn link(&self, source: &Path, destination: &Path, root: bool) -> anyhow::Result<()> {
        let command = format!("ln -s {} {}", source.canonicalize()?.to_string_lossy(), destination.canonicalize()?.to_string_lossy());

        self.run(&command, root, false)
    }

    /// Makes a given file executable
    pub fn make_executable(&self, path: &Path, root: bool) -> anyhow::Result<()> {
        let command = format!("chmod +x {}", path.canonicalize()?.to_string_lossy());
        
        self.run(&command, root, false)
    }
}

const FALLBACK_SHELL: &str = "/bin/sh";

/// Runs a given command on the user's shell, with or without output
fn run(exec: &str, output: bool) -> anyhow::Result<bool> {
    let shell = env::var("SHELL").unwrap_or_else(|_| { warn!("Current shell ($SHELL) is not defined, using {}", FALLBACK_SHELL); FALLBACK_SHELL.to_string()});

    let mut command = Command::new(&shell);
    if output { command.stdout(Stdio::inherit()).stdin(Stdio::inherit()).stderr(Stdio::inherit()); }
    command.arg("-c").arg(exec);

    debug!("Running shell command '{}' on {}", exec, shell);
    let mut result = command.spawn()?;

    Ok(result.wait()?.success())
}