extern crate core;

use std::{env, fs};
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::exit;
use std::thread::sleep;
use std::time::Duration;
use log::{debug, error, info};
use crate::command::{Command, RepositoryCommand, SubCommand};
use crate::config::Config;
use clap::Parser;
use uuid::Uuid;
use crate::module::transaction::change::{AtomicChange, RunChange, WriteChange};
use crate::module::transaction::worker::run::handle_worker;
use crate::module::transaction::worker::WorkerPortal;
use crate::output::logger;
use crate::registry::Registry;
use crate::variables::context::read_context;
use crate::variables::evaluate::evaluate;
use crate::variables::{default_system_variables, generate_magic, load_system, VariableError};

mod command;
mod module;
mod config;
mod output;
mod jobs;
mod registry;
mod variables;

fn main() {
    let command: Command = Command::parse();

    logger::enable_logging(command.verbose);

    debug!("Checking standalone commands...");

    match command.topic {
        SubCommand::Worker { socket, id } => { handle_worker(socket, id); return; }
        _ => {}
    }

    debug!("Loading configuration...");

    // Load config
    let config = match Config::read() {
        Ok(c) => { c }
        Err(e) => {
            error!("Failed to read config: {e:#}");
            exit(-1);
        }
    };

    debug!("Loading sources and modules...");

    // Load registry
    let mut registry = Registry::new(&config);
    if let Err(e) = registry.load() {
        error!("Failed to load registry: {e:#}");
        exit(-1);
    }

    debug!("Loading was successful");

    // Add a padding between loading and action output
    if logger::is_verbose() {
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
