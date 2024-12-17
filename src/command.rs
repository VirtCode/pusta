use clap::{Parser, Subcommand};
use uuid::Uuid;

use crate::schema::schema_dir;

#[derive(Parser)]
#[clap(version, about)]
#[command(disable_help_subcommand = true)]
pub struct Command {
    #[clap(subcommand)]
    pub topic: SubCommand,

    // Enables verbose logging
    #[clap(short, long, global = true)]
    pub verbose: bool
}

#[derive(Subcommand)]
pub enum SubCommand {
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

    /// Updates all modules
    Update {
        /// Only update this module
        module: Option<String>
    },

    /// Internal worker spawn command
    #[command(hide = true)]
    Worker {
        #[arg()]
        socket: Uuid,
        #[arg()]
        id: Uuid
    },

    /// Generate json schema files in the specified directory
    #[command(hide = true)]
    Schema {
        #[arg(short, long, default_value_t = schema_dir())]
        directory: String
    }
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
