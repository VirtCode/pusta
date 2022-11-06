use std::path::PathBuf;

pub enum InstallAction {
    Package {
        name: String,
    },
    File {
        file: String,
        target: PathBuf,
        root: Option<bool>,
        link: Option<bool>
    },
    Script {
        install: String,
        uninstall: Option<String>,
        root: Option<bool>
    }
}