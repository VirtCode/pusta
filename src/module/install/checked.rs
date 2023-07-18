use std::path::Path;
use anyhow::anyhow;
use log::{error, info};
use crate::config::{Config, ConfigSecurity, ConfirmStrategy, PreviewStrategy};
use crate::module::install::shell::Shell;
use crate::output;

pub struct CheckedShell {
    security: ConfigSecurity,
    pub unchecked: Shell
}

impl CheckedShell {

    pub fn new(config: &Config) -> Self {
        Self {
            security: config.security.clone(),
            unchecked: Shell::new(config)
        }
    }

    fn check_file(&self, root: bool, action: &str) -> anyhow::Result<()> {
        let confirm = match self.security.confirm_files {
            ConfirmStrategy::No => { false }
            ConfirmStrategy::Yes => { true }
            ConfirmStrategy::Root => { root }
        };

        let run = !confirm || output::prompt_yn(&format!("Do you want to {action}{}?",
                                               if root { " as root" } else { "" }
        ), true);

        if run { Ok(()) } else { Err(anyhow!("User denied script execution")) }
    }

    fn check_script(&self, root: bool, script: &Path, running_directory: Option<&Path>) -> anyhow::Result<()> {

        let preview = match &self.security.preview_scripts {
            PreviewStrategy::Always => { true }
            PreviewStrategy::Root => { root }
            PreviewStrategy::Never => { false }
            ask => {
                let mut do_ask = true;

                // Match ask root case
                if let PreviewStrategy::AskRoot = ask {
                    if !root {
                        do_ask = false;
                    }
                }

                if do_ask {
                    output::prompt_yn(&format!("Do you want to preview the to be run{} script '{}'?",
                                               if root { " as root" } else { "" },
                                               script.to_string_lossy())
                                      , false)
                } else {
                    false
                }
            }
        };

        if preview {
            self.unchecked.preview(script, running_directory).unwrap_or_else(|e| error!("Failed to preview script '{}', {e}", script.to_string_lossy()))
        }

        // If previewed, a prompt will follow automatically
        let confirm = preview || match self.security.confirm_files {
            ConfirmStrategy::No => { false }
            ConfirmStrategy::Yes => { true }
            ConfirmStrategy::Root => { root }
        };

        let run = !confirm || output::prompt_yn(&format!("Do you want to run the script '{}'{}?",
                                               script.to_string_lossy(),
                                               if root { " as root" } else { "" }
        ), true);

        if run { Ok(()) } else { Err(anyhow!("User denied script execution")) }
    }

    fn check_command(&self, root: bool, command: &str) -> anyhow::Result<()> {
        let confirm = match self.security.confirm_files {
            ConfirmStrategy::No => { false }
            ConfirmStrategy::Yes => { true }
            ConfirmStrategy::Root => { root }
        };

        let run = !confirm || output::prompt_yn(&format!("Do you want to run the command '{command}'{} on your shell?",
                                               if root { " as root" } else { "" }
        ), true);

        if run { Ok(()) } else { Err(anyhow!("User denied command execution")) }

    }

    fn check_package(&self, packages: &Vec<String>) -> anyhow::Result<()> {
        let run = !self.security.confirm_packages || output::prompt_yn(&format!("Do you want to manipulate the package(s) '{}'",
                                                                     packages.join(" ")
        ), true);

        if run { Ok(()) } else { Err(anyhow!("User denied package manipulation")) }
    }

    pub fn install(&self, packages: Vec<String>) -> anyhow::Result<()> {
        self.check_package(&packages)?;

        output::start_shell(&format!("Installing package(s) '{}' over the system package manager", packages.join(" ")));
        let result = self.unchecked.install(packages);
        output::end_shell("Finished installation process");

        result
    }

    pub fn uninstall(&self, packages: Vec<String>) -> anyhow::Result<()> {
        self.check_package(&packages)?;

        output::start_shell(&format!("Uninstalling package(s) '{}' over the system package manager", packages.join(" ")));
        let result = self.unchecked.uninstall(packages);
        output::end_shell("Finished uninstallation process");

        result
    }

    pub fn make_dir(&self, path: &Path, root: bool, running_directory: Option<&Path>) -> anyhow::Result<()> {
        self.check_file(root, &format!("make the directory '{}'", path.to_string_lossy()))?;
        self.unchecked.make_dir(path, root, running_directory)
    }

    pub fn remove(&self, path: &Path, root: bool, running_directory: Option<&Path>) -> anyhow::Result<()> {
        self.check_file(root, &format!("delete the file '{}'", path.to_string_lossy()))?;
        self.unchecked.remove(path, root, running_directory)
    }

    pub fn copy(&self, source: &Path, destination: &Path, root: bool, running_directory: Option<&Path>) -> anyhow::Result<()> {
        self.check_file(root, &format!("copy the file from '{}' to '{}'", source.to_string_lossy(), destination.to_string_lossy()))?;
        self.unchecked.copy(source, destination, root, running_directory)
    }

    pub fn link(&self, source: &Path, destination: &Path, root: bool, running_directory: Option<&Path>) -> anyhow::Result<()> {
        self.check_file(root, &format!("symlink the file from '{}' to '{}'", source.to_string_lossy(), destination.to_string_lossy()))?;
        self.unchecked.link(source, destination, root, running_directory)
    }

    pub fn make_executable(&self, path: &Path, root: bool, running_directory: Option<&Path>) -> anyhow::Result<()> {
        self.check_file(root, &format!("make the file '{}' executable", path.to_string_lossy()))?;
        self.unchecked.make_executable(path, root, running_directory)
    }

    pub fn run_script(&self, path: &Path, root: bool, output: bool, running_directory: Option<&Path>) -> anyhow::Result<()> {
        self.check_script(root, &path, running_directory)?;

        if output {
            output::start_shell(&format!("Running script '{}'", path.to_string_lossy()));
        }

        let result = self.unchecked.run(&path.to_string_lossy(), root, output, running_directory);

        if output {
            output::end_shell("Script finished running");
        }

        result
    }

    pub fn run_command(&self, command: &str, root: bool, output: bool, running_directory: Option<&Path>) -> anyhow::Result<()> {
        self.check_command(root, command)?;

        if output {
            output::start_shell(&format!("Running command '{command}'"));
        }

        let result = self.unchecked.run(command, root, output, running_directory);

        if output {
            output::end_shell("Command finished running");
        }

        result
    }
}