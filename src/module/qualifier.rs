use std::path::Path;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
pub struct ModuleQualifier {
    repository: String,
    /// Name of the directory
    dir: String,
    /// Alias defined in the config
    alias: Option<String>,
    /// Provides defined in the config
    provide: Option<String>
}

impl ModuleQualifier {
    
    pub fn new(repository: String, path: &Path, alias: Option<String>, provide: Option<String>) -> Self {
        Self {
            repository,
            dir: path.file_name().map(|os| os.to_string_lossy().to_string()).expect("This can not happen because a module folder always has a name"),
            alias,
            provide
        }
    }

    /// Returns the repository of the qualifier
    pub fn repository(&self) -> &String {
        &self.repository
    }

    /// Returns whether the module provides the named qualifier
    pub fn does_provide(&self, qualifier: &str) -> bool {

        // Provides module
        if let Some(provide) = &self.provide {
            if provide == qualifier { return true }
        }

        // Is the module
        self.name() == qualifier
    }

    // Returns the full qualifier of that module
    pub fn unique(&self) -> &String {
        &format!("{}/{}", &self.repository, self.name())
    }

    /// Returns qualifying name for module
    pub fn name(&self) -> &String {
        if let Some(alias) = &self.alias {
            alias
        } else {
            &self.dir
        }
    }

    /// Returns alternative providing name
    pub fn provide(&self) -> &Option<String> {
        &self.provide
    }
}

impl PartialEq for ModuleQualifier {
    fn eq(&self, other: &Self) -> bool {
        self.unique() == other.unique()
    }
}