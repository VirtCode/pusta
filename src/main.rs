extern crate core;

use std::{env};
use std::io::{BufReader};
use std::os::unix::io::{RawFd};
use std::path::PathBuf;
use std::process::exit;
use log::{debug, error, info, LevelFilter, warn};
use crate::command::{Command, RepositoryCommand, SubCommand};
use crate::config::Config;
use clap::Parser;
use crate::output::{end_section, logger, start_section};
use crate::output::logger::{disable_indent, enable_indent};
use crate::registry::Registry;

mod command;
mod module;
mod config;
mod output;
mod jobs;
mod registry;

pub const FILE_REPOSITORY: &str = "pusta.yml";
pub const FILE_MODULE: &str = "module.yml";

pub const CACHE_MODULES: &str = "~/.config/pusta/cache/modules.json";
pub const CACHE_REPOSITORIES: &str = "~/.config/pusta/cache/repositories.json";
pub const CACHE: &str = "~/.config/pusta/cache/";

fn main() {
    let command: Command = Command::parse();
    let mut config = Config::read();

    logger::enable_logging(config.log.verbose || command.verbose);

    debug!("Loading sources and modules...");

    // Load registry
    let mut registry = Registry::new(&config);
    if let Err(e) = registry.load() {
        disable_indent();
        error!("Failed to load registry: {}", e.to_string());
        exit(-1);
    }

    debug!("Loading was successful");

    println!();
    match command.topic {
        SubCommand::Source { action } => {
            match action {
                RepositoryCommand::Add { path,  alias }  => {
                    let dir = path.map(|p| PathBuf::from(shellexpand::tilde(&p).to_string())).unwrap_or_else(|| env::current_dir().unwrap());

                    registry.add(&dir, alias.as_deref());
                }
                RepositoryCommand::Remove { alias }  => {

                    registry.unadd(&alias);
                }
            }
        },
        SubCommand::Install { module } => {
            registry.install(&module);
        },
        SubCommand::Remove { module } => {
            registry.remove(&module);
        }
        _ => {}
    }

}
