pub mod shell;

use std::path::{Path, PathBuf};
use anyhow::Error;
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
            InstallAction::Package { names, .. } => {

                let result = shell.install_package(names)?;
                if !result { return Err(Error::msg("package manager failed or the install was canceled by user")) }

                Ok(InstalledAction::Package { installed: names.clone() })
            }
            InstallAction::File { file, target, root, link, .. } => {

                let mut source = origin.to_path_buf();
                source.push(file);
                source = source.canonicalize()?;

                let mut sink = PathBuf::from(shellexpand::tilde(target).as_ref());
                sink = sink.canonicalize()?;

                let result = if link.unwrap_or(false) {
                    shell.create_symlink(file, &source, &sink, root.unwrap_or(false))?
                } else {
                    shell.copy_file(file, &source, &sink, root.unwrap_or(false))?
                };

                if !result { return Err(Error::msg("file action canceled by user")) }

                Ok(InstalledAction::File {root: root.unwrap_or(true), location: sink})
            }
            InstallAction::Script { install, uninstall, complete_reinstall, root, important_output, .. } => {

                let mut path = origin.to_path_buf();
                path.push(install);
                path = path.canonicalize()?;

                let result = shell.execute_script(&path.to_string_lossy(), root.unwrap_or(false), install, important_output.unwrap_or(false))?;
                if !result { return Err(Error::msg("script failed to run, see output")) }

                if let Some(s) = uninstall {
                    Ok(InstalledAction::Script {uninstall: s.clone(), root: root.unwrap_or(false)})
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
        installed: String
    },
    File {
        location: PathBuf,
        root: bool
    },
    Script {
        uninstall: String,
        root: bool
    },
    Empty
}

impl InstalledAction {
    pub fn uninstall(&self, shell: &Shell, cache: &Path) -> anyhow::Result<()>{
        match self {
            InstalledAction::Package { installed } => {

                let result = shell.remove_package(installed)?;
                if !result { return Err(Error::msg("package manager failed or the removal was canceled by user")) }

            }
            InstalledAction::File { location, root } => {

                let result = shell.remove_file(&location, *root)?;
                if !result { return Err(Error::msg("file removal probably canceled by user")) }

            }
            InstalledAction::Script { uninstall, root } => {

                let mut path = cache.to_path_buf();
                path.push(uninstall);

                let result = shell.execute_script(&path.to_string_lossy(), *root, "uninstaller", false)?;
                if !result { return Err(Error::msg("uninstaller script failed to execute properly")) }

            }
            Empty => {}
        }

        Ok(())
    }
}



