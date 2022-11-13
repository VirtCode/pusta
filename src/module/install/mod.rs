pub mod shell;

use std::fs;
use std::path::{Path, PathBuf};
use anyhow::{Context, Error};
use log::info;
use serde::{Deserialize, Serialize};
use crate::module::install::InstalledAction::Empty;
use crate::module::install::shell::Shell;

#[derive(Deserialize, Serialize)]
#[serde(tag = "action")]
pub enum InstallAction {
    #[serde(rename="package")]
    Package {
        title: Option<String>,
        optional: Option<bool>,
        names: String,
    },
    #[serde(rename="file")]
    File {
        title: Option<String>,
        optional: Option<bool>,
        file: String,
        target: String,
        root: Option<bool>,
        link: Option<bool>
    },
    #[serde(rename="script")]
    Script {
        title: Option<String>,
        optional: Option<bool>,
        install: String,
        uninstall: Option<String>,
        important_output: Option<bool>,
        complete_reinstall: Option<bool>,
        root: Option<bool>
    }
}

impl InstallAction {

    pub fn install(&self, shell: &Shell, origin: &Path) -> anyhow::Result<InstalledAction> {

        match self {
            InstallAction::Package { names, title, .. } => {
                info!("Trying to install the packages '{}' over the shell", names.replace(" ", "', '"));

                let result = shell.install_package(names)?;
                if !result { return Err(Error::msg("Package manager failed or the install was canceled by user")) }

                info!("Successfully installed all required packages for this action");

                Ok(InstalledAction::Package { installed: names.clone(), title: title.clone() })
            }
            InstallAction::File { file, target, root, link, title, .. } => {
                info!("{} the file {} to its foreseen location", if link.unwrap_or(false) { "Symlinking" } else { "Copying" }, file);

                let mut source = origin.to_path_buf();
                source.push(file);
                source = source.canonicalize().context("Failed to canonicalize path of source file")?;

                let sink = PathBuf::from(shellexpand::tilde(target).as_ref());
                if let Some(parent) = sink.parent() {
                    if !parent.exists() {
                        info!("Creating parent directory for file to be placed in if not existent");
                        fs::create_dir_all(parent).context("Failed to create parent directories for sink file")?;
                    } else if !parent.is_dir() {
                        return Err(Error::msg("Target parent directory is not a directory"))
                    }
                }

                info!("Required paths evaluated, executing the action over the shell");

                let result = if link.unwrap_or(false) {
                    shell.create_symlink(file, &source, &sink, root.unwrap_or(false))?
                } else {
                    shell.copy_file(file, &source, &sink, root.unwrap_or(false))?
                };

                if !result { return Err(Error::msg("Symlink or copy was cancelled by user or failed otherwise")) }

                info!("Successfully placed that file at its foreseen location");

                Ok(InstalledAction::File {root: root.unwrap_or(false), location: sink, title: title.clone()})
            }
            InstallAction::Script { install, uninstall, complete_reinstall, root, important_output, title, .. } => {

                let mut path = origin.to_path_buf();
                path.push(install);
                path = path.canonicalize()?;

                let result = shell.execute_script(&path.to_string_lossy(), root.unwrap_or(false), install, important_output.unwrap_or(false))?;
                if !result { return Err(Error::msg("script failed to run, see output")) }

                if let Some(s) = uninstall {
                    Ok(InstalledAction::Script {uninstall: s.clone(), root: root.unwrap_or(false), title: title.clone()})
                } else { Ok(Empty) }
            }
        }
    }

    pub fn is_optional(&self) -> bool {
        match self {
            InstallAction::Package { optional, .. } => { optional }
            InstallAction::File { optional, .. } => { optional }
            InstallAction::Script { optional, .. } => { optional }
        }.unwrap_or(false)
    }

    pub fn get_title(&self) -> &Option<String> {
        match self {
            InstallAction::Package { title, .. } => { title }
            InstallAction::File { title, .. } => { title }
            InstallAction::Script { title, .. } => { title }
        }
    }
}

#[derive(Deserialize, Serialize)]
pub enum InstalledAction {
    Package {
        title: Option<String>,
        installed: String
    },
    File {
        title: Option<String>,
        location: PathBuf,
        root: bool
    },
    Script {
        title: Option<String>,
        uninstall: String,
        root: bool
    },
    Empty
}

impl InstalledAction {
    pub fn uninstall(&self, shell: &Shell, cache: &Path) -> anyhow::Result<()>{
        match self {
            InstalledAction::Package { installed, .. } => {

                info!("Removing previously installed package(s) '{}' over the shell", installed.replace(" ", "', '"));
                let result = shell.remove_package(installed)?;
                if !result { return Err(Error::msg("Package manager failed or the removal was canceled by user")) }

            }
            InstalledAction::File { location, root, .. } => {

                info!("Removing installed file from {}{}", location.to_string_lossy(), if *root { ", using root" } else { "" });
                let result = shell.remove_file(location, *root)?;
                if !result { return Err(Error::msg("File removal probably canceled by user")) }

            }
            InstalledAction::Script { uninstall, root, .. } => {

                info!("Running available uninstaller script{}", if *root { "as root" } else { "" });
                let mut path = cache.to_path_buf();
                path.push(uninstall);

                let result = shell.execute_script(&path.to_string_lossy(), *root, "uninstaller", false)?;
                if !result { return Err(Error::msg("Uninstaller script failed to execute properly")) }

            }
            Empty => {}
        }

        Ok(())
    }

    pub fn get_title(&self) -> &Option<String> {
        match self {
            InstalledAction::Package { title, .. } => { title }
            InstalledAction::File { title, .. } => { title }
            InstalledAction::Script { title, .. } => { title }
            _ => { &None }
        }
    }
}



