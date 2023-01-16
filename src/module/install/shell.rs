use std::io::{BufRead, BufReader, Read};
use std::{env, process};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread::sleep;
use std::time::Duration;
use log::{debug, error, info, warn};
use crate::config::{Config, ConfigSecurity, ConfigShell, ConfirmStrategy, PreviewStrategy};
use crate::output;
use crate::output::prompt_yn;

const FALLBACK_SHELL: &str = "/bin/sh";

pub fn run(exec: &str, output: bool) -> anyhow::Result<bool> {
    let shell = env::var("SHELL").unwrap_or_else(|_| { warn!("Current shell ($SHELL) is not defined, using {}", FALLBACK_SHELL); FALLBACK_SHELL.to_string()});

    let mut command = Command::new(&shell);
    if output { command.stdout(Stdio::inherit()).stdin(Stdio::inherit()).stderr(Stdio::inherit()); }
    command.arg("-c").arg(exec);

    debug!("Running shell command '{}' on {}", exec, shell);
    let mut result = command.spawn()?;

    Ok(result.wait()?.success())
}

pub fn run_task(command: &str, message: &str, success_message: &str, failure_message: &str) -> anyhow::Result<bool> {
    output::start_shell(message);

    let result = run(command, true);
    match &result {
        Ok(result) => {
            output::end_shell(*result, if *result {success_message} else {failure_message});
        }
        Err(_) => {
            output::end_shell(false, failure_message);
        }
    }

    result
}

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

    fn run(&self, root: bool, command: &str, task: Option<(String, String, String)>, output: bool) -> anyhow::Result<bool>{
        let command = if root {
            self.shell_config.root_elevator.replace(ROOT_COMMAND_KEY, command)
        } else { command.to_string() };

        if self.security_config.extra_confirm_everything && !output::prompt_yn(&format!("Really run '{}' on your shell?", &command), true) {
            return Ok(false)
        }

        if let Some((message, success, failure)) = task {
            run_task(&command, &message, &success, &failure)
        } else {
            run(&command, output)
        }
    }

    pub fn execute_script(&self, script: &str, root: bool, name: &str, task: bool) -> anyhow::Result<bool> {
        self.preview_script(script, root, name);

        let confirm = match self.security_config.confirm_scripts {
            ConfirmStrategy::No => { false }
            ConfirmStrategy::Yes => { true }
            ConfirmStrategy::Root => { root }
        };

        let run = !confirm || prompt_yn(&format!("Run the script {}{}?", name, if root {" as root"} else {""}), true);

        if run {
            let task_text = if task {
                Some(((format!("Running script {}...", name)), (format!("Successfully ran script {}", name)), (format!("Script {} failed to run properly", name))))
            } else { None };

            self.run(root, script, task_text, task)

        } else {
            Ok(false)
        }
    }

    fn preview_script(&self, file: &str, root: bool, name: &str) {
        let preview = match self.security_config.preview_scripts {
            PreviewStrategy::Always => { true }
            PreviewStrategy::Root => { root }
            PreviewStrategy::Never => { false }
            PreviewStrategy::Ask => {
                prompt_yn(&format!("Preview the next script {}{}?", name, if root {", which is to be run as root"} else {""}), false)
            }
            PreviewStrategy::AskRoot => {
                root && prompt_yn(&format!("Preview the next script {}, which is to be run as root?", name), false)
            }
        };

        if preview {
            info!("Opening preview for file at {}", file);

            if !self.run(false, &self.shell_config.file_previewer.replace(FILE_PREVIEW_KEY, file), None, true).unwrap_or(false) {
                error!("Failed to preview file '{}'", file);
            }
        }
    }

    pub fn install_package(&self, name: Vec<String>) -> anyhow::Result<bool> {
        if self.security_config.confirm_packages && !prompt_yn(&format!("Install the package(s) '{}' over the system package manager?", name.join("', '")), true) {
            return Ok(false)
        }

        let command = self.shell_config.package_manager.install.replace(PACKAGE_COMMAND_KEY, &name.join(" "));
        let task = Some(((format!("Installing system package(s) '{}'...", name.join("', '"))), (format!("Successfully installed system package(s) '{}'", name.join("', '"))), (format!("Failed to install system package(s) '{}'", name.join("', '")))));

        self.run(self.shell_config.package_manager.root, &command, task, true)
    }

    pub fn remove_package(&self, name: Vec<String>) -> anyhow::Result<bool> {
        if self.security_config.confirm_packages && !prompt_yn(&format!("Remove the package(s) '{}' over the system package manager?", name.join("', '")), true) {
            return Ok(false)
        }

        let command = self.shell_config.package_manager.remove.replace(PACKAGE_COMMAND_KEY, &name.join(" "));
        let task = Some(((format!("Removing system package(s) '{}'...", name.join("', '"))), (format!("Successfully removed system package(s) '{}'", name.join("', '"))), (format!("Failed to remove system package(s) '{}'", name.join("', '")))));

        self.run(self.shell_config.package_manager.root, &command, task, true)
    }

    pub fn copy_file(&self, name: &str, source: &Path, sink: &Path, root: bool) -> anyhow::Result<bool> {
        let confirm = match self.security_config.confirm_copy {
            ConfirmStrategy::No => { false }
            ConfirmStrategy::Yes => { true }
            ConfirmStrategy::Root => { root }
        };

        let run = !confirm || prompt_yn(&format!("Copy the file {} to {}{}?", name, sink.to_string_lossy(), if root {" as root"} else {""}), true);

        if run { self.run(root, &format!("cp {} {}", source.to_string_lossy(), sink.to_string_lossy()), None, false) }
        else { Ok(false) }
    }

    pub fn create_symlink(&self, name: &str, source: &Path, sink: &Path, root: bool) -> anyhow::Result<bool> {
        let confirm = match self.security_config.confirm_copy {
            ConfirmStrategy::No => { false }
            ConfirmStrategy::Yes => { true }
            ConfirmStrategy::Root => { root }
        };

        let run = !confirm || prompt_yn(&format!("Symlink the file {} to {}{}?", name, sink.to_string_lossy(), if root {" as root"} else {""}), true);

        if run { self.run(root, &format!("ln -s {} {}", source.to_string_lossy(), sink.to_string_lossy()), None, false) }
        else { Ok(false) }
    }

    pub fn remove_file(&self, target: &Path, root: bool) -> anyhow::Result<bool> {
        let confirm = match self.security_config.confirm_copy {
            ConfirmStrategy::No => { false }
            ConfirmStrategy::Yes => { true }
            ConfirmStrategy::Root => { root }
        };

        let run = !confirm || prompt_yn(&format!("Remove the installed file at {}{}?", target.to_string_lossy(), if root {" as root"} else {""}), true);

        if run { self.run(root, &format!("rm {}", target.to_string_lossy()), None, false) }
        else { Ok(false) }
    }


}