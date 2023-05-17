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

fn main() {
    let command: Command = Command::parse();
    let config = Config::read();

    logger::enable_logging(config.log.verbose || command.verbose);

    debug!("Loading sources and modules...");

    // Load registry
    let mut registry = Registry::new(&config);
    if let Err(e) = registry.load() {
        error!("Failed to load registry: {e:#}");
        exit(-1);
    }

    debug!("Loading was successful");

    if config.log.verbose {
        println!();
    }

    match command.topic {
        SubCommand::Source { action } => {
            match action {
                RepositoryCommand::Add { path,  alias }  => {
                    let dir = path.map(|p| PathBuf::from(shellexpand::tilde(&p).to_string())).unwrap_or_else(|| env::current_dir().expect("not being run in a directory?!?"));
                    registry.add_repository(&dir, alias.as_deref());
                }
                RepositoryCommand::Remove { alias }  => {
                    registry.remove_repository(&alias);
                }
            }
        },
        SubCommand::Install { module } => {
            registry.install_module(&module);
        },
        SubCommand::Remove { module } => {
            registry.uninstall_module(&module);
        },
        SubCommand::List => {
            registry.list();
        },
        SubCommand::Query { module } => {
            registry.query_module(&module);
        },
        SubCommand::Update { module } => {
            match module {
                None => { registry.update_everything() }
                Some(module) => { registry.update_module(&module) }
            }
        }
        _ => {}
    }
}
