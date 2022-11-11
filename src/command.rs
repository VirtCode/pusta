use clap::{Parser, Subcommand};

#[derive(Parser)]
#[clap(version, about)]
#[command(disable_help_subcommand = true)]
pub struct Command {
    #[clap(subcommand)]
    pub topic: SubCommand,

    // Enables verbose logging
    #[clap(short, long)]
    pub verbose: bool
}

#[derive(Subcommand)]
pub enum SubCommand {
    // Loads main repository and uses the provided config
    Load {
        folder: Option<String>
    },

    // Modify the repositories
    Repo {
        #[clap(subcommand)]
        action: RepositoryCommand,
    },

    // Modify the modules
    Module {
        #[clap(subcommand)]
        action: ModuleCommand,
    },

    Reload

}

#[derive(Subcommand)]
pub enum ModuleCommand {
    // Installs a module
    Install {
        module: String
    },

    // Removes a module
    Remove {
        module: String
    },

    // Updates all or specified module(s)
    Update {
        modules: Option<Vec<String>>
    }
}

#[derive(Subcommand)]
pub enum RepositoryCommand {
    // Adds a repository to the sources
    Add {
        path: Option<String>,

        #[clap(short, long)]
        alias: Option<String>
    },

    // Removes a repository with all installed modules
    Remove {
        alias: String
    },

    // Queries or changes the main repository to another repo
    Main {
        alias: Option<String>
    }

}
