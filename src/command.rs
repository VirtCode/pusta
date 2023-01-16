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
    /// Loads main repository and uses the provided config
    Load {
        folder: Option<String>
    },

    /// Installs a module
    Install {
        /// Qualifier of module
        module: String
    },

    /// Uninstalls a module
    Remove {
        /// Qualifier of module
        module: String
    },

    /// Lists added sources and installed modules
    List,

    /// Queries for modules and shows relevant information
    Query {
        /// Qualifier to query for
        module: String
    },

    /// Change the module sources
    Source {
        #[clap(subcommand)]
        action: RepositoryCommand,
    },

    Update

}

#[derive(Subcommand)]
pub enum RepositoryCommand {
    /// Adds a repository to the sources
    Add {
        /// Path of the directory the repository lives
        path: Option<String>,

        /// Custom alias for the repository
        #[clap(short, long)]
        alias: Option<String>
    },

    /// Removes a source without removing its modules
    Remove {
        /// Alias of source to remove
        alias: String
    },
}
