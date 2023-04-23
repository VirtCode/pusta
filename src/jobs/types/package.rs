use std::ops::Deref;
use anyhow::Error;
use dyn_eq::DynEq;
use log::{info, warn};
use serde::{Deserialize, Serialize};
use crate::jobs::{Installable, InstallReader, InstallWriter, JobCacheReader, JobCacheWriter, JobEnvironment};

/// This job installs a package from the system
#[derive(Serialize, Deserialize, Clone, Eq, PartialEq)]
pub struct PackageJob {
    names: String
}

impl PackageJob {
    fn name_vec(&self) -> Vec<String> {
        self.names.split(' ').map(|s| s.to_owned()).collect()
    }
}

#[typetag::serde(name = "package")]
impl Installable for PackageJob {

    fn install(&self, env: &JobEnvironment, writer: &mut InstallWriter) -> anyhow::Result<()> {
        let names = self.name_vec();

        env.shell.install(names)?;

        Ok(())
    }

    fn uninstall(&self, env: &JobEnvironment, reader: &InstallReader) -> anyhow::Result<()> {
        let names = self.name_vec();

        env.shell.uninstall(names)?;

        Ok(())
    }

    fn update(&self, old: &dyn Installable, env: &JobEnvironment, writer: &mut InstallWriter, reader: &InstallReader) -> Option<anyhow::Result<()>> {
        let old = old.as_any().downcast_ref::<Self>()?;

        // Compare packages
        let old = old.name_vec();
        let new = self.name_vec();

        // Remove removed
        let uninstall: Vec<String> = old.iter().filter(|s| !new.contains(*s)).cloned().collect();
        env.shell.uninstall(uninstall).unwrap_or_else(|e| warn!("Couldn't uninstall removed packages properly: {e}"));

        // Install new
        let install: Vec<String> = new.iter().filter(|s| !old.contains(*s)).cloned().collect();
        Some(env.shell.install(install))
    }

    fn construct_title(&self) -> String {
        format!("Installing the package(s) '{}' on the system", self.name_vec().join("', '"))
    }
}