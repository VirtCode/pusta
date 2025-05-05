use std::fs::File;
use std::path::Path;
use anyhow::{anyhow, Context};
use schemars::JsonSchema;
use serde::Deserialize;
use crate::module::repository::Repository;
use crate::variables::Variable;

pub const HOST_CONFIG_FILEENDING: &str = ".host.yml";

#[derive(Deserialize, JsonSchema)]
#[schemars(title = "Host", deny_unknown_fields)]
pub struct HostConfig {
    /// override the hostname for this hosts file, otherwise filename is used
    pub hostname: Option<String>,

    /// modules that should be installed on this host
    pub modules: String,

    /// variables only to be applied on that host
    pub variables: Option<Variable>,
}

pub struct Host {
    pub hostname: String,
    pub repository: String,

    pub modules: Vec<String>,
    pub variables: Option<Variable>
}

impl Host {
    pub fn try_load(file: &Path, repository: &Repository) -> anyhow::Result<Self> {
        let filename = file.file_name()
            .context("file doesn't have a name")?
            .to_string_lossy();

        let hostname = filename.strip_suffix(HOST_CONFIG_FILEENDING)
                    .context("file is not a host file")?;

        let config: HostConfig = serde_yaml::from_reader(File::open(&file).context("Failed to open host file")?)
            .map_err(|f| anyhow!("Failed to read host file ({})", f.to_string()))?;

        let hostname = config.hostname.unwrap_or(hostname.to_owned());

        Ok(Self {
            hostname,
            repository: repository.name.clone(),
            modules: config.modules.split_ascii_whitespace().map(|a| a.to_string()).collect::<Vec<_>>(),
            variables: config.variables
        })
    }
}
