use anyhow::Error;
use log::info;
use serde::{Deserialize, Serialize};
use crate::jobs::{Installable, InstallReader, InstallWriter, JobCacheReader, JobCacheWriter, JobEnvironment};

/// This job installs a package from the system
#[derive(Serialize, Deserialize, Clone)]
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

        info!("Using configured package manager for install");

        env.shell.install(names)?;

        info!("Successfully installed all required packages");
        Ok(())
    }

    fn uninstall(&self, env: &JobEnvironment, reader: &InstallReader) -> anyhow::Result<()> {
        let names = self.name_vec();

        info!("Removing previously installed packages");

        env.shell.uninstall(names)?;

        Ok(())
    }

    fn construct_title(&self) -> String {
        format!("Installing the package(s) '{}' on the system", self.name_vec().join("', '"))
    }
}