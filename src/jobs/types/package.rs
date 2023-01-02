use anyhow::Error;
use log::info;
use serde::{Deserialize, Serialize};
use crate::jobs::{Installable, InstallReader, InstallWriter, JobCacheReader, JobCacheWriter, JobEnvironment};

/// This job installs a package from the system
#[derive(Serialize, Deserialize)]
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

        info!("Trying to install the packages '{}' over the shell", names.join("', '"));

        env.shell.install(names)?;

        info!("Successfully installed all required packages for this action");
        Ok(())
    }

    fn uninstall(&self, env: &JobEnvironment, reader: &InstallReader) -> anyhow::Result<()> {
        let names = self.name_vec();

        info!("Removing previously installed package(s) '{}' over the shell", names.join("', '"));

        env.shell.install(names)?;

        Ok(())
    }

    fn construct_title(&self) -> String {
        format!("install the packages '{}' on the system", self.name_vec().join("', '"))
    }
}