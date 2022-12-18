extern crate core;

use std::{env};
use std::io::{BufReader};
use std::os::unix::io::{RawFd};
use std::path::PathBuf;
use std::process::exit;
use log::{error, info, LevelFilter, warn};
use crate::command::{Command, ModuleCommand, RepositoryCommand, SubCommand};
use crate::config::Config;
use crate::manager::Manager;
use clap::Parser;
use crate::module::install::shell;
use crate::module::install::shell::Shell;
use crate::output::{logger};

mod command;
mod module;
mod manager;
mod config;
mod output;
mod jobs;

fn main() {
    let command: Command = Command::parse();
    let mut config = Config::read();

    logger::enable_logging(config.log.log_files, config.log.verbose || command.verbose);

    let mut manager = Manager::load(&config);

    println!();

    match command.topic {
        SubCommand::Module { action } => {

            match action {
                ModuleCommand::Install { module } => {

                    let shell = Shell::new(&config);

                    match manager.install_module(&module, &shell) {
                        Ok(result) => { exit(if result { 1 } else { 0 }) }
                        Err(e) => {
                            error!("Failed to manipulate cache: {}", e);
                            exit(1)
                        }
                    };

                }
                ModuleCommand::Remove { module } => {

                    let shell = Shell::new(&config);
                    match manager.uninstall_module(&module, &shell) {
                        Ok(result) => { exit(if result { 1 } else { 0 }) }
                        Err(e) => {
                            error!("Failed to manipulate cache: {}", e);
                            exit(1)
                        }
                    };

                }
                ModuleCommand::Update { modules } => {}
            }

        },
        SubCommand::Repo { action } => {
            match action {
                RepositoryCommand::Add { path,  alias }  => {

                    // Add repository
                    let dir = if let Some(path) = path {
                        PathBuf::from(path)
                    } else {
                        env::current_dir().unwrap()

                    };

                    manager.add_repository(&dir, alias.as_ref()).unwrap();


                }
                RepositoryCommand::Remove { alias }  => {

                    // Remove repository
                    manager.remove_repository(&alias).unwrap();

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
