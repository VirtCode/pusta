extern crate core;

use std::{env};
use std::io::{BufReader};
use std::os::unix::io::{RawFd};
use std::path::PathBuf;
use std::process::exit;
use log::{error, info, LevelFilter, warn};
use crate::command::{Command, ModuleCommand, RepositoryCommand, SubCommand};
use crate::config::Config;
use clap::Parser;
use crate::module::install::shell;
use crate::module::install::shell::Shell;
use crate::output::{logger};
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
pub const CACHE_DATA: &str = "~/.config/pusta/cache/data/";

fn main() {
    let command: Command = Command::parse();
    let mut config = Config::read();

    logger::enable_logging(config.log.log_files, config.log.verbose || command.verbose);

    let mut registry = Registry::new(&config);
    registry.read_modules();
    registry.read_repositories().unwrap();

    println!();

    match command.topic {
        SubCommand::Module { action } => {

            match action {
                ModuleCommand::Install { module } => {

                    let shell = Shell::new(&config);

                    install_module(&module, &registry);


                    // match manager.install_module(&module, &shell) {
                    //     Ok(result) => { exit(if result { 1 } else { 0 }) }
                    //     Err(e) => {
                    //         error!("Failed to manipulate cache: {}", e);
                    //         exit(1)
                    //     }
                    // };

                }
                ModuleCommand::Remove { module } => {

                    let shell = Shell::new(&config);
                    // match manager.uninstall_module(&module, &shell) {
                    //     Ok(result) => { exit(if result { 1 } else { 0 }) }
                    //     Err(e) => {
                    //         error!("Failed to manipulate cache: {}", e);
                    //         exit(1)
                    //     }
                    // };

                }
                ModuleCommand::Update { modules } => {}
            }

        },
        SubCommand::Repo { action } => {
            match action {
                RepositoryCommand::Add { path,  alias }  => {

                    let dir = path.map(PathBuf::from).unwrap_or_else(|| env::current_dir().unwrap());

                    registry.add(&dir, alias.as_deref()).unwrap();
                }
                RepositoryCommand::Remove { alias }  => {

                    // Remove repository
                    registry.unadd(&alias);

                }
                RepositoryCommand::Main { alias } => {

                    if let Some(a) = alias {
                        // Set new main repository
                        config.repositories.main = Some(a);
                        config.write();
                    } else {
                        // Get main repository
                        if let Some(current) = &config.repositories.main { println!("The current main repository is '{}'", current); }
                        else { println!("There is no main repository set"); }
                    }

                }
            }
        },
        _ => {}
    }

}

fn install_module(name: &str, registry: &Registry) {
    let names = registry.query(name);

    let name =
        if names.len() == 1 { names.get(0).unwrap().clone() }
        else { output::prompt_choice("Which package do you mean?", names.iter().collect(), None) };

}
